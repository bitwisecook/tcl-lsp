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

//! Profile-aware availability queries over registry data.
//!
//! [`DialectProfile`] lives in the foundational `tcl-dialect` crate, below
//! this one, so it cannot know about [`CommandSpec`] / [`CommandRegistry`].
//! This extension trait gives the profile its availability API (design doc
//! §5.1) at the registry layer: **every** availability consumer resolves
//! commands through these methods so the subtractive iRules disable list
//! (§9) is applied uniformly after the mask query — a bare mask query
//! (`get_for_dialect` alone) would silently re-admit the banned commands
//! once the Milestone 5 data retag lands.

use tcl_dialect::{DialectProfile, DialectSet};

use crate::hover::OptionSpec;
use crate::registry::CommandRegistry;
use crate::spec::{CommandSpec, SubCommand};

/// Availability queries a resolved [`DialectProfile`] answers against
/// registry data (design doc §5.1/§5.2). Implemented for `DialectProfile`
/// here because the spec types live above the foundational crate.
pub trait ProfileQueries {
    /// Whether `spec` is available under this profile: the membership test
    /// against [`DialectProfile::availability_mask`] **and** the subtractive
    /// disable filter ([`DialectProfile::is_command_disabled`], §9).
    fn is_available(&self, spec: &CommandSpec) -> bool;

    /// Resolve `name` to its command spec under this profile — the single
    /// availability primitive the diagnostics (W123/W002), completion,
    /// and the CLI snapshot share. Mask query + disable filter.
    fn resolve_command<'r>(
        &self,
        registry: &'r CommandRegistry,
        name: &str,
    ) -> Option<&'r CommandSpec>;

    /// Whether `sub` (of `spec`) is available under this profile: the
    /// subcommand's own dialect gate — falling back to the parent
    /// command's — intersected with the availability mask (§5.1).
    fn is_subcommand_available(&self, spec: &CommandSpec, sub: &SubCommand) -> bool;

    /// The subcommands of `spec` available under this profile, in
    /// declaration order — the filtered iterator completion was missing
    /// (§5.1's `available_subcommands`).
    fn available_subcommands<'r>(&self, spec: &'r CommandSpec) -> Vec<&'r SubCommand>;

    /// Whether `opt` is available under this profile, given the gate it
    /// inherits from its parent (`spec.dialects`, or
    /// `sub.dialects.or(spec.dialects)` for a subcommand option).
    ///
    /// This is the §5.2 option-gating semantics — a genuine change from
    /// the old `contains` rule:
    ///
    /// - **membership** — `gate.intersects(availability_mask)`, so an
    ///   option inherited from a vendor command resolves under that
    ///   vendor's composed profile (the old `contains` silently dropped
    ///   every inherited option on every vendor command), and
    /// - **upper bound** — the gate's [`DialectSet::min_version`] must not
    ///   exceed [`DialectProfile::version_ceiling`], so a tcl9.0-only
    ///   option cannot leak into an 8.5-superset profile whose mask
    ///   happens to intersect its gate.
    ///
    /// A gate of `None` on both the option and its parent means "no
    /// restriction".
    fn is_option_available(&self, opt: &OptionSpec, parent_gate: Option<DialectSet>) -> bool;

    /// Declared option / switch names of `spec` available under this
    /// profile, in declaration order with duplicates removed — the
    /// profile-aware `switch_names` (§5.1's `available_options`).
    fn available_option_names(&self, spec: &CommandSpec) -> Vec<&'static str>;

    /// The [`OptionSpec`]s of `spec` available under this profile, in
    /// declaration order. (`'static`: spec option tables are interned.)
    fn available_option_specs(&self, spec: &CommandSpec) -> Vec<&'static OptionSpec>;

    /// Option / switch names of subcommand `sub`, whose options inherit
    /// `sub.dialects.or(spec.dialects)` as their parent gate.
    fn available_sub_option_names(&self, spec: &CommandSpec, sub: &SubCommand)
    -> Vec<&'static str>;

    /// The [`OptionSpec`]s of subcommand `sub` available under this
    /// profile (same inheritance as
    /// [`Self::available_sub_option_names`]).
    fn available_sub_option_specs(
        &self,
        spec: &CommandSpec,
        sub: &SubCommand,
    ) -> Vec<&'static OptionSpec>;

    /// Look up an option of `spec` by canonical name or alias, honouring
    /// this profile's gate (§5.2) and the resolved `package_version`.
    fn find_option<'r>(
        &self,
        spec: &'r CommandSpec,
        option_name: &str,
        package_version: Option<&str>,
    ) -> Option<&'r OptionSpec>;
}

impl ProfileQueries for DialectProfile {
    fn is_available(&self, spec: &CommandSpec) -> bool {
        spec.supports_dialect(self.availability_mask) && !self.is_command_disabled(spec.name)
    }

    fn resolve_command<'r>(
        &self,
        registry: &'r CommandRegistry,
        name: &str,
    ) -> Option<&'r CommandSpec> {
        registry
            .get_for_dialect(name, self.availability_mask)
            .filter(|spec| !self.is_command_disabled(spec.name))
    }

    fn is_subcommand_available(&self, spec: &CommandSpec, sub: &SubCommand) -> bool {
        sub.dialects
            .or(spec.dialects)
            .is_none_or(|gate| gate.intersects(self.availability_mask))
    }

    fn available_subcommands<'r>(&self, spec: &'r CommandSpec) -> Vec<&'r SubCommand> {
        spec.subcommands
            .iter()
            .filter(|sub| self.is_subcommand_available(spec, sub))
            .collect()
    }

    fn is_option_available(&self, opt: &OptionSpec, parent_gate: Option<DialectSet>) -> bool {
        let Some(gate) = opt.dialects.or(parent_gate) else {
            // No restriction on the option or its parent.
            return true;
        };
        if !gate.intersects(self.availability_mask) {
            return false;
        }
        match (gate.min_version(), self.version_ceiling) {
            (Some(min), Some(ceiling)) => min <= ceiling,
            // A pure vendor gate has no version floor; a profile without a
            // ceiling accepts every version.
            _ => true,
        }
    }

    fn available_option_names(&self, spec: &CommandSpec) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let mut consider = |opt: &OptionSpec| {
            if self.is_option_available(opt, spec.dialects) && !names.contains(&opt.name) {
                names.push(opt.name);
            }
        };
        for opt in spec.options {
            consider(opt);
        }
        for form in spec.command_forms {
            for opt in form.options {
                consider(opt);
            }
        }
        names
    }

    fn available_option_specs(&self, spec: &CommandSpec) -> Vec<&'static OptionSpec> {
        let mut out: Vec<&'static OptionSpec> = Vec::new();
        let mut consider = |opt: &'static OptionSpec| {
            if self.is_option_available(opt, spec.dialects)
                && !out.iter().any(|o| o.name == opt.name)
            {
                out.push(opt);
            }
        };
        for opt in spec.options {
            consider(opt);
        }
        for form in spec.command_forms {
            for opt in form.options {
                consider(opt);
            }
        }
        out
    }

    fn available_sub_option_names(
        &self,
        spec: &CommandSpec,
        sub: &SubCommand,
    ) -> Vec<&'static str> {
        let parent = sub.dialects.or(spec.dialects);
        let mut names: Vec<&'static str> = Vec::new();
        for opt in sub.options {
            if self.is_option_available(opt, parent) && !names.contains(&opt.name) {
                names.push(opt.name);
            }
        }
        names
    }

    fn available_sub_option_specs(
        &self,
        spec: &CommandSpec,
        sub: &SubCommand,
    ) -> Vec<&'static OptionSpec> {
        let parent = sub.dialects.or(spec.dialects);
        sub.options
            .iter()
            .filter(|opt| self.is_option_available(opt, parent))
            .collect()
    }

    fn find_option<'r>(
        &self,
        spec: &'r CommandSpec,
        option_name: &str,
        package_version: Option<&str>,
    ) -> Option<&'r OptionSpec> {
        let matches = |opt: &&'r OptionSpec| {
            opt.matches(option_name)
                && self.is_option_available(opt, spec.dialects)
                && opt.available_for_version(package_version)
        };
        spec.options.iter().find(matches).or_else(|| {
            spec.command_forms
                .iter()
                .flat_map(|f| f.options.iter())
                .find(matches)
        })
    }
}
