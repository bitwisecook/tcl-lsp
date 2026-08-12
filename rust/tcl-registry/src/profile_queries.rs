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
//! commands through these methods, so the mask query and the profile-level
//! operator-head exclusion (§9) are applied uniformly. iRules availability
//! is fully explicit in each spec's `dialects` group, so a sandbox-banned
//! command such as `exec` simply never carries the `IRULES` bit; there is
//! no subtractive disable list left to bypass.

use tcl_dialect::{DialectProfile, DialectSet};

use crate::hover::OptionSpec;
use crate::registry::CommandRegistry;
use crate::spec::{CommandSpec, SubCommand};
use crate::traits::Traits;

/// Availability queries a resolved [`DialectProfile`] answers against
/// registry data (design doc §5.1/§5.2). Implemented for `DialectProfile`
/// here because the spec types live above the foundational crate.
pub trait ProfileQueries {
    /// Whether `spec` is available under this profile: the membership test
    /// against [`DialectProfile::availability_mask`], and — for a profile
    /// whose math operators are not command heads (`f5-irules`) — the
    /// [`Traits::OPERATOR_COMMAND`] exclusion. iRules availability is fully
    /// explicit in each spec's `dialects` now (the `IRULES` bit is present
    /// iff iRules enables it), so there is no longer a subtractive ban list.
    ///
    /// This is the same trio [`CommandRegistry::spec_visible`] enforces
    /// inside profile-stamped registries; it lives here too so profile-side
    /// queries agree with mask queries on any registry.
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

    /// The profile's **own vendor command surface**, summarised from
    /// registry data (§5.4 "out-of-registry vendor knowledge"): the
    /// available commands carrying this profile's vendor bit, grouped by
    /// `NS::` namespace prefix (bare names group under `""`), sorted by
    /// descending size then name. `None` for profiles with no vendor
    /// surface of their own. Feeds generated consumers (the AI prompt's
    /// F5-surface summary) so prose can never drift from the data.
    fn vendor_surface(&self, registry: &CommandRegistry) -> Option<VendorSurface>;

    /// The **declared version range** of `spec` on this profile's keyed
    /// library axis, when `spec` belongs to one of the profile's ambient
    /// `Keyed` pins (the F5 surfaces): the explicit introduction release,
    /// or the axis **baseline** (BIG-IP 15.0 — "the data is declared
    /// against 15.0+") when none is recorded, plus the removal release
    /// (`None` = still present). `None` for specs outside a keyed pin —
    /// their plain `min_version` semantics are untouched.
    fn keyed_version_range(
        &self,
        spec: &CommandSpec,
    ) -> Option<(Option<&'static str>, Option<&'static str>)>;

    /// The ambient `Keyed` [`tcl_dialect::LibraryPin`] `spec` belongs to
    /// under this profile: by its `owning_package`, or — for a
    /// **vendor-own** spec (its gate carries the profile's vendor bit and
    /// no plain-Tcl version, the same membership rule
    /// [`ProfileQueries::vendor_surface`] uses) — the profile's single
    /// ambient keyed pin, so the F5 packs need no per-spec
    /// `required_package` backfill to sit on the version axis.
    fn keyed_pin_for(&self, spec: &CommandSpec) -> Option<&'static tcl_dialect::LibraryPin>;
}

/// A profile's vendor command surface summary (see
/// [`ProfileQueries::vendor_surface`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorSurface {
    /// Total vendor-tagged commands available under the profile.
    pub command_count: usize,
    /// `(namespace_prefix, command_count)` per `NS::` prefix, sorted by
    /// descending count then name; bare (un-namespaced) commands group
    /// under an empty prefix.
    pub namespaces: Vec<(String, usize)>,
}

/// The set of package names some dialect profile ships **ambient** — an EDA
/// vendor tool surface (`vivado`, `synopsys-dc`, …), an F5 command pack —
/// computed once from the catalog. A command gated on such a package is a
/// closed-world vendor command: available only under a profile that ships it.
/// Hosted libraries that need `package require` (Tk, tcllib, the stdlib
/// packages) are never in this set (design doc `eda-library-packages.md`).
fn vendor_ambient_packages() -> &'static std::collections::HashSet<&'static str> {
    static SET: std::sync::OnceLock<std::collections::HashSet<&'static str>> =
        std::sync::OnceLock::new();
    SET.get_or_init(|| {
        DialectProfile::all()
            .iter()
            .flat_map(|p| p.libraries.iter())
            .filter(|pin| pin.ambient)
            .map(|pin| pin.package)
            .collect()
    })
}

/// Whether `required` is satisfied under `profile` (design doc
/// `eda-library-packages.md`, the `is_available` package-loaded gate):
///
/// - `None` — no package gate, always satisfied.
/// - a **hosted** library (never ambient in any profile — Tk, tcllib, stdlib):
///   satisfied. The command stays known/available; a missing `package require`
///   is reported separately by W120, not by hiding the command.
/// - a **closed-world vendor** package (ambient in some profile): satisfied
///   only when *this* profile ships it ambient — so a `vivado` command never
///   resolves under a plain-Tcl or rival-vendor profile.
///
/// Behaviour-preserving today (vendor packs are profile-scoped and each EDA
/// profile ships its own tool packages ambient); the gate makes the model
/// honest and future-proofs finer per-tool detection.
fn package_available(profile: &DialectProfile, required: Option<&'static str>) -> bool {
    match required {
        None => true,
        Some(pkg) => profile.is_ambient_package(pkg) || !vendor_ambient_packages().contains(pkg),
    }
}

impl ProfileQueries for DialectProfile {
    fn is_available(&self, spec: &CommandSpec) -> bool {
        spec.supports_dialect(self.availability_mask)
            && (self.operators_as_commands || !spec.traits.contains(Traits::OPERATOR_COMMAND))
            && package_available(self, spec.required_package)
    }

    fn resolve_command<'r>(
        &self,
        registry: &'r CommandRegistry,
        name: &str,
    ) -> Option<&'r CommandSpec> {
        registry
            .get_for_dialect(name, self.availability_mask)
            .filter(|spec| self.is_available(spec))
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

    fn keyed_version_range(
        &self,
        spec: &CommandSpec,
    ) -> Option<(Option<&'static str>, Option<&'static str>)> {
        let pin = self.keyed_pin_for(spec)?;
        let tcl_dialect::LibraryVersion::Keyed(key) = pin.version else {
            return None;
        };
        let lifecycle = spec.lifecycle.with_baseline(key.baseline_version());
        Some((lifecycle.introduced, lifecycle.retired))
    }

    fn keyed_pin_for(&self, spec: &CommandSpec) -> Option<&'static tcl_dialect::LibraryPin> {
        let keyed_ambient = |pin: &&'static tcl_dialect::LibraryPin| {
            pin.ambient && matches!(pin.version, tcl_dialect::LibraryVersion::Keyed(_))
        };
        if let Some(pkg) = spec.owning_package()
            && let Some(pin) = self.library_pin(pkg)
        {
            return keyed_ambient(&pin).then_some(pin);
        }
        let vendor = self.vendor_bit?;
        let vendor_own = spec
            .dialects
            .is_some_and(|d| d.intersects(vendor) && !d.intersects(DialectSet::ALL_TCL));
        if !vendor_own {
            return None;
        }
        self.libraries.iter().find(keyed_ambient)
    }

    fn vendor_surface(&self, registry: &CommandRegistry) -> Option<VendorSurface> {
        let vendor = self.vendor_bit?;
        let mut by_ns: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        let mut command_count = 0usize;
        for name in registry.command_names() {
            let Some(spec) = self.resolve_command(registry, name) else {
                continue;
            };
            // Vendor-OWN means the vendor bit is the discriminating tag: a
            // gate that also spans the plain Tcl versions is shared library
            // data (tcllib's complement-shaped "everywhere but the closed
            // sandboxes" gates), not the vendor's own surface.
            if !spec
                .dialects
                .is_some_and(|d| d.intersects(vendor) && !d.intersects(DialectSet::ALL_TCL))
            {
                continue;
            }
            command_count += 1;
            let ns = name.split_once("::").map_or("", |(ns, _)| ns);
            *by_ns.entry(ns.to_owned()).or_insert(0) += 1;
        }
        if command_count == 0 {
            return None;
        }
        let mut namespaces: Vec<(String, usize)> = by_ns.into_iter().collect();
        namespaces.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Some(VendorSurface {
            command_count,
            namespaces,
        })
    }
}
