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

//! Documentation and completion metadata for LSP features.

use crate::dialects::DialectSet;

/// Short hover content derived from man pages or vendor docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoverSnippet {
    /// One-line summary.
    pub summary: &'static str,
    /// Invocation synopsis lines (e.g. `"for start test next body"`).
    pub synopsis: &'static [&'static str],
    /// Extended description.
    pub snippet: &'static str,
    /// Documentation source (e.g. `"Tcl for(1)"`).
    pub source: &'static str,
    /// Usage examples.
    pub examples: &'static str,
    /// Return value description.
    pub return_value: &'static str,
}

impl HoverSnippet {
    /// A hover with only summary, synopsis, and source — the common case.
    #[must_use]
    pub const fn brief(
        summary: &'static str,
        synopsis: &'static [&'static str],
        source: &'static str,
    ) -> Self {
        Self {
            summary,
            synopsis,
            snippet: "",
            source,
            examples: "",
            return_value: "",
        }
    }
}

/// Completion and hover metadata for a positional argument value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentValueSpec {
    /// Completable value text.
    pub value: &'static str,
    /// Short description in the completion list.
    pub detail: &'static str,
}

/// Metadata for a switch-like option (`-nonewline`, `-nocase`, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSpec {
    /// Option name (e.g. `"-nonewline"`).
    pub name: &'static str,
    /// Whether this option consumes a following value.
    pub takes_value: bool,
    /// Hint text for the value (e.g. `"channel"`).
    pub value_hint: &'static str,
    /// Short description.
    pub detail: &'static str,
    /// Dialect membership.  `None` means "inherit from the parent
    /// `CommandSpec` / `SubCommand` dialects" — the common case.
    /// Set this to restrict an option added in a specific Tcl
    /// version (e.g. `lsearch -stride` is Tcl 8.6+, `clock scan
    /// -validate` is Tcl 9.0+) so the option doesn't surface in
    /// older dialects.
    pub dialects: Option<DialectSet>,
    /// Documented alternate spellings Tcl accepts for this same option
    /// (e.g. `-bd` for `-borderwidth`, `-bg` for `-background`).  These are
    /// *explicit* aliases the command's own option table recognises — not the
    /// general unambiguous-prefix matching Tcl also allows.  Validation,
    /// value-arity, and option lookup treat an alias exactly like `name`;
    /// completion offers only the canonical `name`.
    pub aliases: &'static [&'static str],
    /// Minimum *package* version that introduced this option, as a dotted Tcl
    /// version string (e.g. `entry -placeholder` needs Tk `8.7`).  `None`
    /// means "present in every version of the owning package".  Gated against
    /// the version resolved from `package require` — orthogonal to `dialects`
    /// (which gates on the Tcl *core* version).
    pub min_version: Option<&'static str>,
}

impl OptionSpec {
    /// Check whether this option is available in *dialect*.
    ///
    /// If the option has its own `dialects` set, use it.  Otherwise
    /// inherit from *`parent_dialects`* (the parent `CommandSpec` or
    /// `SubCommand`).  When either side is `None`, the option is
    /// considered available (no restriction).
    #[must_use]
    pub fn supports_dialect(
        &self,
        dialect: Option<DialectSet>,
        parent_dialects: Option<DialectSet>,
    ) -> bool {
        let Some(active) = dialect else {
            return true;
        };
        if let Some(own) = self.dialects {
            return own.contains(active);
        }
        let Some(parent) = parent_dialects else {
            return true;
        };
        parent.contains(active)
    }

    /// Whether `option_name` is this option's canonical name or an alias.
    #[must_use]
    pub fn matches(&self, option_name: &str) -> bool {
        self.name == option_name || self.aliases.contains(&option_name)
    }

    /// Whether this option exists given the resolved *`package_version`*.
    ///
    /// *`package_version`* is the guaranteed-available floor derived from a
    /// `package require` (see [`crate::version::requirement_lower_bound`]).
    /// `None` (no version constraint known) is permissive; an option with no
    /// `min_version` is always available.
    #[must_use]
    pub fn available_for_version(&self, package_version: Option<&str>) -> bool {
        match (self.min_version, package_version) {
            (Some(min), Some(have)) => crate::version::meets_min(have, min),
            _ => true,
        }
    }
}

/// Completion / hover metadata for a single enumerable
/// positional-argument value.
///
/// Used for arguments
/// whose value comes from a fixed set — e.g. the character
/// class in `string is <class>`, the event name in iRules
/// `when <EVENT>`, or a subcommand keyword.  The completion
/// provider surfaces `value` (with `detail` as the right-hand
/// description) when the cursor sits on the matching argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgValue {
    /// The literal value (e.g. `"alnum"`).
    pub value: &'static str,
    /// Short description for the completion list.
    pub detail: &'static str,
}

/// Classification of a command invocation form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormKind {
    /// Default form.
    Default,
    /// Getter form (read-only).
    Getter,
    /// Setter form (modifying).
    Setter,
}

/// A concrete invocation form of a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormSpec {
    /// Form classification.
    pub kind: FormKind,
    /// Human-readable invocation signature.
    pub synopsis: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_dialect_inherits_from_parent_when_unset() {
        let opt = OptionSpec {
            name: "-foo",
            takes_value: false,
            value_hint: "",
            detail: "",
            dialects: None,
            aliases: &[],
            min_version: None,
        };
        // No parent: always available.
        assert!(opt.supports_dialect(Some(DialectSet::TCL84), None));
        // Parent allows everything: available.
        assert!(opt.supports_dialect(Some(DialectSet::TCL84), Some(DialectSet::ALL_TCL)));
        // Parent restricts: inherit the restriction.
        assert!(opt.supports_dialect(Some(DialectSet::TCL86), Some(DialectSet::TCL86_PLUS)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL85), Some(DialectSet::TCL86_PLUS)));
    }

    #[test]
    fn supports_dialect_own_set_overrides_parent() {
        // `lsearch -stride` is Tcl 8.6+ even though `lsearch` itself
        // is available since 8.4.  The option's own dialects field
        // wins.
        let opt = OptionSpec {
            name: "-stride",
            takes_value: true,
            value_hint: "int",
            detail: "",
            dialects: Some(DialectSet::TCL86_PLUS),
            aliases: &[],
            min_version: None,
        };
        assert!(opt.supports_dialect(Some(DialectSet::TCL86), Some(DialectSet::ALL_TCL)));
        assert!(opt.supports_dialect(Some(DialectSet::TCL90), Some(DialectSet::ALL_TCL)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL84), Some(DialectSet::ALL_TCL)));
        assert!(!opt.supports_dialect(Some(DialectSet::TCL85), Some(DialectSet::ALL_TCL)));
    }

    #[test]
    fn supports_dialect_none_active_is_unrestricted() {
        // No active dialect = treat option as available (e.g.
        // unscoped completion).
        let opt = OptionSpec {
            name: "-x",
            takes_value: false,
            value_hint: "",
            detail: "",
            dialects: Some(DialectSet::TCL90),
            aliases: &[],
            min_version: None,
        };
        assert!(opt.supports_dialect(None, Some(DialectSet::TCL90)));
    }
}
