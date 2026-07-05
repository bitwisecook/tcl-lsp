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

//! Command registry — lookup facade.
//!
//! Built once at startup from command spec modules, then queried by
//! every consumer. Supports dialect filtering and trait-membership
//! queries.

use std::collections::HashMap;
use std::sync::OnceLock;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::dialects::DialectSet;
use crate::forms::CommandForm;
use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::spec::{BytePayloadSpec, CommandSpec, SubCommand};
use crate::traits::Traits;

/// Resolved metadata for an iRules event — the result of
/// [`CommandRegistry::event_info`].
#[derive(Debug, Clone)]
pub struct EventInfo {
    /// The upper-cased event name as queried.
    pub event: String,
    /// Whether the event is a recognised iRules event.
    pub known: bool,
    /// Whether the event is deprecated (always `false`; see
    /// [`CommandRegistry::event_info`]).
    pub deprecated: bool,
    /// `"init"` / `"once_per_connection"` / `"per_request"` / `"unknown"`.
    pub multiplicity: &'static str,
    /// Description prose, or `""` when none is recorded.
    pub description: String,
    /// Connection-side label, or `"unknown"` for an unrecognised event.
    pub side: &'static str,
    /// Transport string (`"tcp"`, `"tcp/udp"`, `""`), or `None` for an
    /// unrecognised event.
    pub transport: Option<String>,
    /// Profile types implied by the event, sorted.
    pub implied_profiles: Vec<&'static str>,
    /// Sorted names of every command valid in this event (empty when the
    /// event is unknown).
    pub valid_commands: Vec<String>,
}

impl EventInfo {
    /// Number of commands valid in this event.
    #[must_use]
    pub fn valid_command_count(&self) -> usize {
        self.valid_commands.len()
    }
}

/// Number of commands declaring a taint source — computed at compile
/// time so [`TAINT_SOURCE_INDEX`] can be a fixed-size `const` array.
const fn count_taint_sources(specs: &[CommandSpec]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < specs.len() {
        if specs[i].taint_source.is_some() {
            n += 1;
        }
        i += 1;
    }
    n
}

/// Build the taint-source index at compile time by scanning the const
/// [`crate::commands::irules::IRULES_SPECS`] array for every spec's
/// [`crate::CommandSpec::taint_source`].
const fn build_taint_source_index()
-> [(&'static str, crate::taint::TaintColour); TAINT_SOURCE_COUNT] {
    let specs = crate::commands::irules::IRULES_SPECS;
    let mut out = [("", crate::taint::TaintColour::empty()); TAINT_SOURCE_COUNT];
    let mut i = 0;
    let mut k = 0;
    while i < specs.len() {
        if let Some(colour) = specs[i].taint_source {
            out[k] = (specs[i].name, colour);
            k += 1;
        }
        i += 1;
    }
    out
}

const TAINT_SOURCE_COUNT: usize = count_taint_sources(crate::commands::irules::IRULES_SPECS);

/// The taint-source index: command name → getter-form source colour, a
/// **compile-time** table derived from every iRules spec's
/// [`crate::CommandSpec::taint_source`] — the data's single home is each
/// `CommandSpec`, so this never drifts from the spec definitions.
///
/// Independent of which dialects a registry has loaded: a `tcl8.6`
/// document still
/// sees `HTTP::path` as a source. (The core Tcl sources `gets` / `read` /
/// `exec` / … are classified by [`crate::Traits::TAINT_SOURCE`] instead,
/// so they carry no index entry.)
const TAINT_SOURCE_INDEX: [(&str, crate::taint::TaintColour); TAINT_SOURCE_COUNT] =
    build_taint_source_index();

/// Lookup facade over command specs.
///
/// The registry is built once from the command spec modules and then
/// queried read-only. All command-specific knowledge lives in the
/// specs — consumers never match on command name strings.
pub struct CommandRegistry {
    by_name: FxHashMap<String, Vec<CommandSpec>>,
    loaded_dialects: DialectSet,
}

/// The set of command names registered by *every* dialect, built once and
/// cached.  Backs [`CommandRegistry::known_in_any_dialect`] — the
/// dialect-agnostic existence check over every loaded dialect.
/// Built from the same spec functions [`CommandRegistry::build_default`]
/// and [`CommandRegistry::load_dialect`] draw from, so it stays in lock-step
/// with the registry's command universe.
fn all_dialect_command_names() -> &'static FxHashSet<&'static str> {
    static NAMES: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let mut set: FxHashSet<&'static str> = FxHashSet::default();
        let mut add = |specs: Vec<CommandSpec>| {
            for spec in specs {
                set.insert(spec.name);
            }
        };
        add(crate::commands::bpf::bpf_command_specs());
        add(crate::commands::tcl::tcl_command_specs());
        add(crate::commands::stdlib::stdlib_command_specs());
        add(crate::commands::tcllib::tcllib_command_specs());
        add(crate::commands::argparse::argparse_command_specs());
        add(crate::commands::tk::tk_command_specs());
        add(crate::commands::irules::irules_command_specs());
        add(crate::commands::iapps::iapps_command_specs());
        add(crate::commands::expect::expect_command_specs());
        add(crate::commands::sdc_base::sdc_base_command_specs());
        add(crate::commands::eda_synopsys::eda_synopsys_command_specs());
        add(crate::commands::eda_cadence::eda_cadence_command_specs());
        add(crate::commands::eda_xilinx::eda_xilinx_command_specs());
        add(crate::commands::eda_quartus::eda_quartus_command_specs());
        add(crate::commands::eda_mentor::eda_mentor_command_specs());
        set
    })
}

impl CommandRegistry {
    /// Build the default registry with core Tcl + stdlib + tcllib commands.
    #[must_use]
    pub fn build_default() -> Self {
        let mut registry = Self {
            by_name: FxHashMap::default(),
            loaded_dialects: DialectSet::empty(),
        };
        for spec in crate::commands::tcl::tcl_command_specs() {
            registry.insert(spec);
        }
        for spec in crate::commands::stdlib::stdlib_command_specs() {
            registry.insert(spec);
        }
        for spec in crate::commands::tcllib::tcllib_command_specs() {
            registry.insert(spec);
        }
        for spec in crate::commands::argparse::argparse_command_specs() {
            registry.insert(spec);
        }
        // Tk geometry/widget commands (`grid` / `pack` / `wm` / `button` / …)
        // are part of the always-known command universe: a `.tcl` script may
        // `package require Tk` at runtime, and the diagnostics treat them as
        // recognised under every Tcl dialect, so Tk is folded into the base
        // registry.  Mark the dialect loaded so a later `load_dialect(TK)` is
        // a no-op rather than a double-insert.
        for spec in crate::commands::tk::tk_command_specs() {
            registry.insert(spec);
        }
        registry.loaded_dialects |= DialectSet::TK;
        registry
    }

    /// Load a dialect's commands into the registry (idempotent).
    pub fn load_dialect(&mut self, dialect: DialectSet) {
        if self.loaded_dialects.contains(dialect) {
            return;
        }
        let specs: Vec<CommandSpec> = match dialect {
            d if d == DialectSet::BPF => crate::commands::bpf::bpf_command_specs(),
            d if d == DialectSet::IRULES => crate::commands::irules::irules_command_specs(),
            d if d == DialectSet::IAPPS => crate::commands::iapps::iapps_command_specs(),
            d if d == DialectSet::TK => crate::commands::tk::tk_command_specs(),
            d if d == DialectSet::EXPECT => crate::commands::expect::expect_command_specs(),
            d if d == DialectSet::SYNOPSYS => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_synopsys::eda_synopsys_command_specs());
                v
            }
            d if d == DialectSet::CADENCE => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_cadence::eda_cadence_command_specs());
                v
            }
            d if d == DialectSet::XILINX => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_xilinx::eda_xilinx_command_specs());
                v
            }
            d if d == DialectSet::QUARTUS => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_quartus::eda_quartus_command_specs());
                v
            }
            d if d == DialectSet::MENTOR => {
                let mut v = crate::commands::sdc_base::sdc_base_command_specs();
                v.extend(crate::commands::eda_mentor::eda_mentor_command_specs());
                v
            }
            _ => Vec::new(),
        };
        for spec in specs {
            self.insert(spec);
        }
        self.loaded_dialects |= dialect;
    }

    /// Load iRules dialect commands (convenience wrapper).
    pub fn load_irules(&mut self) {
        self.load_dialect(DialectSet::IRULES);
    }

    /// Load BPF-Tcl dialect commands (convenience wrapper).
    pub fn load_bpf(&mut self) {
        self.load_dialect(DialectSet::BPF);
    }

    /// Whether this registry's dialect reads a bare leading-zero integer
    /// (`08`, `010`) as **octal**.
    ///
    /// Tcl 9.0 dropped the leading-zero octal rule (TIP 472): `08` parses as
    /// decimal 8 and `010` as decimal 10. Every earlier Tcl (8.4/8.5/8.6) and
    /// every 8.x-derived dialect (f5-irules ≈ 8.4, f5-iapps ≈ 8.5/8.6, the EDA
    /// dialects) keeps the octal rule, where `08`/`09` are *invalid* octal
    /// (treated as a string in `==`/`!=`) and `010` is 8.
    ///
    /// The per-dialect registry built by `registry_for_dialect` records its
    /// Tcl version via [`Self::load_dialect`], so the only registry whose
    /// `loaded_dialects` carries [`DialectSet::TCL90`] is the tcl9.0 one; every
    /// other dialect (including the F5/EDA registries, which never load a Tcl
    /// version bit) reads leading zeros as octal.
    #[must_use]
    pub fn leading_zero_is_octal(&self) -> bool {
        !self.loaded_dialects.contains(DialectSet::TCL90)
    }

    /// Insert a command spec into the registry.
    pub fn insert(&mut self, spec: CommandSpec) {
        self.by_name
            .entry(spec.name.to_owned())
            .or_default()
            .push(spec);
    }

    /// Whether `name` exists as a command in *any* dialect, independent of
    /// which dialects this registry instance loaded.
    ///
    /// Looks up the global by-name index across every dialect's specs.
    /// Like [`Self::taint_source`], this is deliberately dialect-agnostic:
    /// an iRules command such as `when` is "known" even when analysing a
    /// `tcl8.6` document whose registry never loaded the iRules specs — the
    /// W002 disabled-in-dialect check needs to distinguish "exists, but not
    /// in this dialect" (→ DISALLOWED) from "exists nowhere" (→ W123's
    /// concern).  A leading `::` falls back to the bare name, matching
    /// [`Self::get`].
    #[must_use]
    pub fn known_in_any_dialect(&self, name: &str) -> bool {
        let bare = name.strip_prefix("::").unwrap_or(name);
        all_dialect_command_names().contains(bare)
    }

    /// Look up a command spec by name (dialect-agnostic).
    ///
    /// A leading `::` (global qualifier) falls back to the bare name, so an
    /// explicitly-global call to a built-in (`::foreach`, `::for`, …)
    /// resolves to the same spec as its unqualified form.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CommandSpec> {
        self.by_name
            .get(name)
            .or_else(|| {
                name.strip_prefix("::")
                    .and_then(|bare| self.by_name.get(bare))
            })
            .and_then(|v| v.last())
    }

    /// Look up a command spec filtered by dialect.
    ///
    /// As with [`Self::get`], a leading `::` falls back to the bare name.
    #[must_use]
    pub fn get_for_dialect(&self, name: &str, dialect: DialectSet) -> Option<&CommandSpec> {
        self.by_name
            .get(name)
            .or_else(|| {
                name.strip_prefix("::")
                    .and_then(|bare| self.by_name.get(bare))
            })
            .and_then(|specs| specs.iter().rev().find(|s| s.supports_dialect(dialect)))
    }

    /// Return all registered command names.
    pub fn command_names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }

    /// Whether `pkg` is a package the registry knows about — i.e. at
    /// least one registered command declares it as its
    /// [`required_package`](crate::CommandSpec::required_package).
    ///
    /// Used by the W120 (missing-`package require`) check: a `package
    /// require` of an *unknown* third-party package may itself pull in
    /// arbitrary commands (e.g. a wrapper that `package require Tk`s
    /// internally), so the analyser cannot prove a Tk/extension command
    /// is unprovided and must suppress W120.
    #[must_use]
    pub fn provides_package(&self, pkg: &str) -> bool {
        self.by_name
            .values()
            .flat_map(|specs| specs.iter())
            .any(|spec| spec.required_package == Some(pkg))
    }

    /// Return every registered [`CommandSpec`] for `name` (all dialects),
    /// in registration order. Empty when the name is unknown.
    ///
    /// Used by the registry-snapshot builder's order-independent
    /// `resolve_spec`.
    #[must_use]
    pub fn specs(&self, name: &str) -> &[CommandSpec] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }

    /// The taint-source colour declared by `command`'s spec, or `None`
    /// when it is not a source.
    ///
    /// Reads the compile-time [`TAINT_SOURCE_INDEX`] — a table *derived*
    /// from every command spec's [`crate::CommandSpec::taint_source`],
    /// built at compile time. It is deliberately **dialect-agnostic and
    /// independent of which dialects are loaded into this registry**:
    /// an iRules
    /// getter such as `HTTP::path` is a known source even when analysing a
    /// `tcl8.6` document whose registry never loaded the iRules commands.
    #[must_use]
    pub fn taint_source(&self, command: &str) -> Option<crate::taint::TaintColour> {
        TAINT_SOURCE_INDEX
            .iter()
            .find(|(name, _)| *name == command)
            .map(|(_, colour)| *colour)
    }

    /// Return the names of `command`'s subcommands whose traits include
    /// `t` — the subcommand-level counterpart of [`Self::commands_with_trait`].
    /// Empty when the command is unknown or has no matching subcommand.
    #[must_use]
    pub fn subcommands_with_trait(&self, command: &str, t: Traits) -> Vec<&str> {
        self.get(command).map_or_else(Vec::new, |spec| {
            spec.subcommands
                .iter()
                .filter(|s| s.traits.contains(t))
                .map(|s| s.name)
                .collect()
        })
    }

    /// Return the sorted names of every f5-irules command valid in `event`.
    ///
    /// The valid-command set: a command
    /// is valid when it supports the iRules dialect, the event is not in its
    /// `excluded_events`, and either it carries no `event_requires` or the
    /// event's [`crate::events::EventProps`] satisfy them.  Returns an empty
    /// vector for an unknown event.
    #[must_use]
    pub fn valid_irules_commands_for_event<'a>(
        &'a self,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> Vec<&'a str> {
        let Some(props) = events.get_props(event) else {
            return Vec::new();
        };
        let mut names: Vec<&str> = self
            .by_name
            .iter()
            .filter_map(|(name, specs)| {
                // Best spec for the dialect — reversed so curated overrides
                // win, matching `get_for_dialect` / `get`.
                let spec = specs
                    .iter()
                    .rev()
                    .find(|s| s.supports_dialect(DialectSet::IRULES))?;
                if spec.excluded_events.contains(&event) {
                    return None;
                }
                if let Some(req) = spec.event_requires.as_ref()
                    && !crate::events::event_satisfies(props, req, event, profiles)
                {
                    return None;
                }
                Some(name.as_str())
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// Whether `command` is legal in iRules `event` — the O(1) legality-matrix
    /// test.
    ///
    /// A command is legal when the event is known, the command supports the
    /// iRules dialect, the event is not in the command's `excluded_events`,
    /// and the command's [`crate::events::EventRequires`] (if any) are
    /// satisfied by the event's [`crate::events::EventProps`].  An unknown
    /// event is illegal for every command — an event with no props has an
    /// empty valid-command set.
    #[must_use]
    pub fn is_irules_command_legal_in_event(
        &self,
        command: &str,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> bool {
        let Some(props) = events.get_props(event) else {
            return false;
        };
        let Some(spec) = self.get_for_dialect(command, DialectSet::IRULES) else {
            return false;
        };
        if spec.excluded_events.contains(&event) {
            return false;
        }
        spec.event_requires
            .as_ref()
            .is_none_or(|req| crate::events::event_satisfies(props, req, event, profiles))
    }

    /// Sorted iRules events where `command` is legal — the inverse of
    /// [`Self::valid_irules_commands_for_event`].  Used by the IRULE1001
    /// "Available in: …" hint.
    #[must_use]
    pub fn irules_events_for_command<'a>(
        &self,
        command: &str,
        events: &'a crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> Vec<&'a str> {
        let mut names: Vec<&str> = events
            .all_event_names()
            .into_iter()
            .filter(|event| self.is_irules_command_legal_in_event(command, event, events, profiles))
            .collect();
        names.sort_unstable();
        names
    }

    /// Resolve full metadata for an iRules `event` — the engine behind
    /// `f5 irule event-info`.
    ///
    /// The event name is upper-cased and trimmed. `known` /
    /// `valid_commands` come from this registry (the cross-product is the
    /// same as [`Self::valid_irules_commands_for_event`]); the side /
    /// transport / implied-profiles / description / multiplicity come from
    /// `events`. `deprecated` is always `false` — the `when`-argument-value
    /// detail path carries no "deprecated" markers (independent of
    /// [`crate::events::EventProps::deprecated`]).
    #[must_use]
    pub fn event_info(
        &self,
        event: &str,
        events: &crate::events::EventRegistry,
        profiles: &crate::profiles::ProfileRegistry,
    ) -> EventInfo {
        let name = event.trim().to_uppercase();
        let known = !name.is_empty() && events.is_known(&name);
        let valid_commands: Vec<String> = if known {
            self.valid_irules_commands_for_event(&name, events, profiles)
                .into_iter()
                .map(ToOwned::to_owned)
                .collect()
        } else {
            Vec::new()
        };
        let props = events.get_props(&name);
        EventInfo {
            known,
            deprecated: false,
            multiplicity: events.multiplicity(&name),
            description: events.description(&name).unwrap_or("").to_owned(),
            side: props.map_or("unknown", crate::events::EventProps::side_label),
            // "No transport" is modelled as `None`, not an empty string.
            transport: props.and_then(|p| (!p.transport.is_empty()).then(|| p.transport.join("/"))),
            implied_profiles: {
                let mut v: Vec<&'static str> = props
                    .map(|p| p.implied_profiles.to_vec())
                    .unwrap_or_default();
                v.sort_unstable();
                v
            },
            event: name,
            valid_commands,
        }
    }

    /// Return all command specs whose traits include `t`.
    #[must_use]
    pub fn commands_with_trait(&self, t: Traits) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .last()
                    .filter(|s| s.traits.contains(t))
                    .map(|_| name.as_str())
            })
            .collect()
    }

    /// Whether `name` is a core Tcl built-in carrying the
    /// [`Traits::BYTE_COMPILED`] trait — i.e. the minifier must not
    /// rewrite this command head to a `$var` alias.  Checks every
    /// registered spec for the name (not just the dialect-preferred
    /// one) so the core stamp is honoured even when a dialect layers
    /// an additional spec under the same name.
    #[must_use]
    pub fn is_byte_compiled(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::BYTE_COMPILED))
        })
    }

    /// Whether `name` is unsafe in sandboxed dialects — it allows
    /// context escalation (`uplevel`, `history`).  Drives the IRULE2003
    /// "unsafe iRules command" check.  Checks every spec registered
    /// under the name.
    #[must_use]
    pub fn is_unsafe(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.unsafe_command))
    }

    /// Whether `name` is valid only at the top level of an iRule script
    /// (`when`, `proc`, `priority`, `timing`).  Drives the IRULE5006 /
    /// IRULE5007 placement checks ([`Traits::IRULES_TOP_LEVEL_ONLY`]).
    #[must_use]
    pub fn is_irules_top_level_only(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::IRULES_TOP_LEVEL_ONLY))
        })
    }

    /// Whether `name` should appear as a notable action node in a flow
    /// diagram ([`Traits::DIAGRAM_ACTION`]). Accepts both the bare
    /// (`HTTP::respond`) and the
    /// canonical (`::HTTP::respond`) spelling — the leading `::` stamped on
    /// `IRCall.canonical_command` by lowering is stripped to recover the
    /// bare registration form — and reflects the dialects loaded into this
    /// registry (the diagram-action set is part of the per-registry trait
    /// index, so a `--dialect f5-irules` registry recognises iRules
    /// actions).
    #[must_use]
    pub fn is_diagram_action(&self, name: &str) -> bool {
        let has = |n: &str| {
            self.by_name.get(n).is_some_and(|specs| {
                specs
                    .iter()
                    .any(|s| s.traits.contains(Traits::DIAGRAM_ACTION))
            })
        };
        has(name) || name.strip_prefix("::").is_some_and(has)
    }

    /// Whether `name` is explicitly marked **never** translatable to F5
    /// Distributed Cloud (XC) — any registered spec whose
    /// [`CommandSpec::xc_translatable`](crate::CommandSpec) is
    /// `Some(false)`. Consumed by the `f5-xc` iRule→XC translator.
    #[must_use]
    pub fn is_xc_never_translatable(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.xc_translatable == Some(false)))
    }

    /// Whether `name` is explicitly marked translatable to XC despite an
    /// otherwise-untranslatable namespace prefix — any registered spec
    /// whose [`CommandSpec::xc_translatable`](crate::CommandSpec) is
    /// `Some(true)`.
    #[must_use]
    pub fn is_xc_translatable_override(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|specs| specs.iter().any(|s| s.xc_translatable == Some(true)))
    }

    /// Whether `name` carries the [`Traits::NOT_PROC_FACTORY`] trait —
    /// a registered command head that incidentally matches the
    /// proc-factory token shape but is not a factory wrapper.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_not_proc_factory(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::NOT_PROC_FACTORY))
        })
    }

    /// Whether `name` is an iRules side-switch — `clientside`,
    /// `serverside`, or `peer` — i.e. a command that evaluates its
    /// nesting-script body under a different connection-side context
    /// than the surrounding event ([`Traits::IS_SIDE_SWITCH`]).
    /// Consulted by the iRules collect/release/payload flow check when
    /// it descends into a side-switch body.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under
    /// the name.
    #[must_use]
    pub fn is_side_switch(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.contains(Traits::IS_SIDE_SWITCH))
        })
    }

    /// Resolve argument indices for a given role.
    ///
    /// For subcommand-based commands (e.g. `dict create`), pass the
    /// subcommand as the first element of `args`. This is the Rust
    /// equivalent of `arg_indices_for_role()`.
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, args: &[&str], role: ArgRole) -> Vec<usize> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let n = args.len();

        // Check subcommand
        if !spec.subcommands.is_empty()
            && !args.is_empty()
            && let Some(sub) = spec.subcommand(args[0])
        {
            // Try dynamic resolver first
            if let Some(resolver) = sub.arg_role_resolver {
                return resolver(&args[1..])
                    .into_iter()
                    .filter(|(_, r)| *r == role)
                    .map(|(i, _)| i as usize + 1) // +1 for subcommand word
                    .filter(|&idx| idx < n)
                    .collect();
            }
            // Static roles (offset by +1 for subcommand word)
            return sub
                .arg_roles
                .iter()
                .filter(|(_, r)| *r == role)
                .map(|(i, _)| *i as usize + 1)
                .filter(|&idx| idx < n)
                .collect();
        }

        // Top-level: try dynamic resolver first
        if let Some(resolver) = spec.arg_role_resolver {
            return resolver(args)
                .into_iter()
                .filter(|(_, r)| *r == role)
                .map(|(i, _)| i as usize)
                .filter(|&idx| idx < n)
                .collect();
        }

        // Static roles — filter by args length to avoid out-of-range indices
        spec.arg_roles
            .iter()
            .filter(|(_, r)| *r == role)
            .map(|(i, _)| *i as usize)
            .filter(|&idx| idx < n)
            .collect()
    }

    /// Resolve a concrete call to its registry-described form.
    ///
    /// Given the command head `name` and the literal argument words
    /// `args`, returns a [`ResolvedCall`] describing which
    /// [`CommandSpec`] / [`SubCommand`] / [`CommandForm`] the call
    /// matches and which lowering / codegen hook applies. The
    /// returned reference borrows from the registry, so callers must
    /// not retain it across registry mutation.
    ///
    /// Returns `None` when the command is unknown to the registry.
    #[must_use]
    pub fn resolve_call<'r>(
        &'r self,
        name: &str,
        args: &[&str],
        dialect: DialectSet,
    ) -> Option<ResolvedCall<'r>> {
        let spec = if dialect.is_empty() {
            self.get(name)?
        } else {
            self.get_for_dialect(name, dialect)?
        };

        let mut resolved = ResolvedCall {
            spec,
            sub: None,
            form: None,
            lowering_hook: spec.lowering_hook,
            codegen_hook: spec.codegen_hook,
        };

        if !spec.subcommands.is_empty()
            && let Some(first) = args.first()
            && let Some(sub) = spec.subcommand(first)
        {
            // Re-slice rather than allocating a fresh `Vec<&str>`
            // — `resolve_call` is on the lowering / codegen /
            // analysis hot path.
            let sub_args: &[&str] = args.get(1..).unwrap_or(&[]);
            let form = pick_form(sub.subcommand_forms, sub_args, dialect);
            resolved.sub = Some(sub);
            resolved.lowering_hook = form
                .and_then(|f| f.lowering_hook)
                .or(sub.lowering_hook)
                .or(spec.lowering_hook);
            resolved.codegen_hook = form
                .and_then(|f| f.codegen_hook)
                .or(sub.codegen_hook)
                .or(spec.codegen_hook);
            resolved.form = form;
            return Some(resolved);
        }

        let form = pick_form(spec.command_forms, args, dialect);
        if let Some(f) = form {
            resolved.lowering_hook = f.lowering_hook.or(spec.lowering_hook);
            resolved.codegen_hook = f.codegen_hook.or(spec.codegen_hook);
            resolved.form = Some(f);
        }
        Some(resolved)
    }

    /// Resolve the option-terminator profile for a command invocation.
    ///
    /// Matches the invocation's first argument against subcommands
    /// that declare an [`OptionSpec`](crate::hover::OptionSpec) with
    /// `name == "--"`, then falls back to form-level `--` declarations.
    /// Returns `None` when the command does not declare a `--`
    /// terminator at all (subcommand-scoped or form-scoped).
    ///
    /// Drives the W304 ("missing option terminator") diagnostic — the
    /// returned `scan_start` index, `subcommand` label, and
    /// `options` slice tell the caller where to start scanning for
    /// the first positional argument and which option specs to
    /// consult for value-consuming options (so they're not mistaken
    /// for positionals).  Returning a borrowed `&'static [OptionSpec]`
    /// rather than a freshly-allocated `HashSet` keeps the resolver
    /// allocation-free on the analyser hot path; per-command option
    /// counts are small (typically 1-3 value-consuming options per
    /// command), so a linear scan at the call site is cheaper than
    /// a `HashSet` build.
    ///
    /// `warn_without_terminator` lifts the
    /// [`Traits::WARN_WITHOUT_TERMINATOR`] flag from the matched
    /// command spec and surfaces it on `ResolvedTerminator`, but the
    /// current W304 emitter does not consume it.  Kept on the resolver
    /// for future emit logic and so the registry API doesn't need to
    /// change when consumers start gating on it.
    #[must_use]
    pub fn resolve_option_terminator(
        &self,
        name: &str,
        args: &[&str],
        dialect: DialectSet,
    ) -> Option<ResolvedTerminator> {
        let spec = if dialect.is_empty() {
            self.get(name)?
        } else {
            self.get_for_dialect(name, dialect)?
        };

        let warn_flag = spec.traits.contains(Traits::WARN_WITHOUT_TERMINATOR);

        // Subcommand-scoped first.
        if let Some(first) = args.first()
            && let Some(sub) = spec.subcommand(first)
            && sub.options.iter().any(|o| o.name == "--")
        {
            return Some(ResolvedTerminator {
                scan_start: 1,
                subcommand: Some(sub.name),
                options: sub.options,
                warn_without_terminator: warn_flag,
            });
        }

        // Form-level fallback — option specs live at the
        // `CommandSpec.options` level (a single set per spec), so we
        // consult that directly when no subcommand match was found.
        if spec.options.iter().any(|o| o.name == "--") {
            return Some(ResolvedTerminator {
                scan_start: 0,
                subcommand: None,
                options: spec.options,
                warn_without_terminator: warn_flag,
            });
        }

        None
    }

    /// Whether `name` (or the compound key `"cmd sub"`) produces a
    /// canonical Tcl list — a list whose elements are properly
    /// quoted so re-parsing by ``eval`` / ``uplevel`` /
    /// ``interp eval`` doesn't trigger unwanted substitution.
    ///
    /// The canonical-list-command set is
    /// derived from `return_type == TclType::List` on the command
    /// (or its subcommand entry), minus the explicit exclusion
    /// `concat` whose join-strip-grouping semantics can leave
    /// unquoted specials in the output.
    ///
    /// Drives the W101 (`eval` with string concatenation) safe-
    /// idiom suppression — `eval [list ...]`, `eval [linsert ...]`,
    /// `eval [split ...]`, etc. shouldn't fire because the inner
    /// command's output is a properly-quoted list.
    #[must_use]
    pub fn is_canonical_list_command(&self, name: &str) -> bool {
        // Exclusion: concat returns LIST but isn't canonical.
        if name == "concat" {
            return false;
        }
        // Compound form ``"cmd sub"`` — split into head + sub.
        if let Some((head, sub_name)) = name.split_once(' ') {
            if let Some(spec) = self.get(head)
                && let Some(sub) = spec.subcommand(sub_name)
            {
                return sub.return_type == Some(crate::types::TclType::List);
            }
            return false;
        }
        // Bare command name.
        self.get(name)
            .and_then(|spec| spec.return_type)
            .is_some_and(|t| t == crate::types::TclType::List)
    }

    /// `{command: BytePayloadSpec}` for every registered `<proto>::payload`
    /// byte-array command — the getter is a binary source and `<cmd> replace`
    /// a byte sink for the S110 byte-array-corruption check.
    ///
    /// Only commands actually loaded into this registry are returned, so the
    /// set is implicitly gated by the active dialect: a plain-Tcl registry
    /// (no iRules pack loaded) yields an empty map — the payload layouts
    /// are intersected with the active dialect.
    #[must_use]
    pub fn byte_array_payload_layouts(&self) -> HashMap<&'static str, BytePayloadSpec> {
        let mut out = HashMap::new();
        for specs in self.by_name.values() {
            for spec in specs {
                if let Some(layout) = spec.byte_array_payload {
                    out.insert(spec.name, layout);
                }
            }
        }
        out
    }

    /// Number of registered commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

/// Resolved option-terminator profile for a command invocation.
///
/// Returned by [`CommandRegistry::resolve_option_terminator`].  Drives
/// the W304 ("missing option terminator") diagnostic.  Carries
/// borrowed references into the registry's static spec table; do
/// not retain across registry mutation.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTerminator {
    /// Index in the `args` slice where positional-argument scanning
    /// begins.  `0` for form-level matches; `1` for subcommand-scoped
    /// matches (the first arg is the subcommand keyword).
    pub scan_start: usize,
    /// Subcommand keyword that owns the `--` declaration, if the
    /// match was subcommand-scoped.  `None` for form-level matches.
    pub subcommand: Option<&'static str>,
    /// Borrowed slice of every option declared on the matched
    /// command (or subcommand).  Callers consult [`crate::hover::OptionSpec::takes_value`]
    /// on each entry to determine whether an option name consumes a
    /// following value argument — done at the call site to avoid the
    /// per-resolve `HashSet` allocation a precomputed name set would
    /// require.  Per-command counts are small; a linear scan is
    /// cheaper than a heap-allocated set on the analyser hot path.
    pub options: &'static [crate::hover::OptionSpec],
    /// Lifted from [`Traits::WARN_WITHOUT_TERMINATOR`] on the matched
    /// command spec.  The current W304
    /// emitter does not consume the flag (it is stored but never read).
    /// Kept on the
    /// type so future emit logic can gate on it without an API
    /// change.
    pub warn_without_terminator: bool,
}

/// Outcome of [`CommandRegistry::resolve_call`].
///
/// Carries borrowed references into the registry's spec table; the
/// resolved call describes the matched command spec, optionally a
/// matched subcommand and form, and the effective lowering / codegen
/// hook identifiers (form-level wins over subcommand-level wins
/// over command-level).
#[derive(Debug, Clone, Copy)]
pub struct ResolvedCall<'r> {
    /// The matched top-level command spec.
    pub spec: &'r CommandSpec,
    /// The matched subcommand, if the call has one.
    pub sub: Option<&'r SubCommand>,
    /// The matched form descriptor, if any.
    pub form: Option<&'r CommandForm>,
    /// Effective lowering hook identifier.
    pub lowering_hook: Option<LoweringHookId>,
    /// Effective codegen hook identifier.
    pub codegen_hook: Option<CodegenHookId>,
}

impl ResolvedCall<'_> {
    /// Effective arity for this resolved call: the form arity if a
    /// form matched, otherwise the subcommand arity, otherwise the
    /// top-level [`CommandSpec`] arity.
    #[must_use]
    pub fn arity(&self) -> Arity {
        if let Some(f) = self.form {
            return f.arity;
        }
        if let Some(s) = self.sub {
            return s.arity;
        }
        self.spec.arity
    }
}

fn pick_form<'r>(
    forms: &'r [CommandForm],
    args: &[&str],
    dialect: DialectSet,
) -> Option<&'r CommandForm> {
    let n = u16::try_from(args.len()).unwrap_or(u16::MAX);
    forms.iter().find(|f| {
        if !f.arity.accepts(n) {
            return false;
        }
        match f.dialects {
            Some(d) if !dialect.is_empty() => d.intersects(dialect),
            _ => true,
        }
    })
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("commands", &self.by_name.len())
            .field("loaded_dialects", &self.loaded_dialects)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_default_has_commands() {
        let reg = CommandRegistry::build_default();
        assert!(!reg.is_empty());
        assert!(reg.get("for").is_some());
        assert!(reg.get("set").is_some());
        assert!(reg.get("nonexistent_command").is_none());
    }

    #[test]
    fn leading_zero_is_octal_tracks_tcl_version() {
        use crate::dialects::DialectSet;
        // Plain default registry (no Tcl version bit) defaults to octal.
        assert!(CommandRegistry::build_default().leading_zero_is_octal());
        // tcl9.0 (TIP 472) reads leading zeros as decimal; everything else
        // (8.4/8.5/8.6 and the 8.x-derived F5 dialects) stays octal.
        let octal_cases = [
            DialectSet::TCL84,
            DialectSet::TCL85,
            DialectSet::TCL86,
            DialectSet::IRULES,
            DialectSet::IAPPS,
        ];
        for d in octal_cases {
            let mut reg = CommandRegistry::build_default();
            reg.load_dialect(d);
            assert!(reg.leading_zero_is_octal(), "{d:?} should be octal");
        }
        let mut reg90 = CommandRegistry::build_default();
        reg90.load_dialect(DialectSet::TCL90);
        assert!(!reg90.leading_zero_is_octal(), "tcl9.0 should be decimal");
    }

    #[test]
    fn irules_command_legality_matrix() {
        use crate::dialects::DialectSet;
        use crate::events::EventRegistry;
        use crate::profiles::ProfileRegistry;
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::IRULES);
        let events = EventRegistry::build();
        let profiles = ProfileRegistry::build();
        // HTTP::respond is satisfied in HTTP_REQUEST (HTTP profile implied) but
        // not in the L4 CLIENT_ACCEPTED event.
        assert!(reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "HTTP_REQUEST",
            &events,
            &profiles
        ));
        assert!(!reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "CLIENT_ACCEPTED",
            &events,
            &profiles
        ));
        // An unknown event is illegal for every command.
        assert!(!reg.is_irules_command_legal_in_event(
            "HTTP::respond",
            "NOT_AN_EVENT",
            &events,
            &profiles
        ));
        // HA::status is explicitly excluded from RULE_INIT.
        assert!(!reg.is_irules_command_legal_in_event(
            "HA::status",
            "RULE_INIT",
            &events,
            &profiles
        ));

        // The inverse list is sorted and reflects the same matrix.
        let evs = reg.irules_events_for_command("HTTP::respond", &events, &profiles);
        assert!(evs.contains(&"HTTP_REQUEST"));
        assert!(!evs.contains(&"CLIENT_ACCEPTED"));
        assert!(
            evs.windows(2).all(|w| w[0] <= w[1]),
            "events must be sorted"
        );
    }

    #[test]
    fn get_resolves_global_qualifier_to_builtin() {
        let reg = CommandRegistry::build_default();
        assert!(reg.get("::foreach").is_some());
        assert_eq!(
            reg.get("::for").map(|s| s.name),
            reg.get("for").map(|s| s.name)
        );
        assert!(reg.get("::nonexistent_command").is_none());
    }

    #[test]
    fn switch_names_is_dialect_filtered() {
        use crate::dialects::DialectSet;
        let reg = CommandRegistry::build_default();
        let regsub = reg.get("regsub").expect("regsub spec");
        // `-command` is Tcl 9.0+ (TIP 463); the always-available
        // switches appear in every dialect.
        let in_86 = regsub.switch_names(Some(DialectSet::TCL86));
        assert!(in_86.contains(&"-all"), "{in_86:?}");
        assert!(in_86.contains(&"-nocase"), "{in_86:?}");
        assert!(
            !in_86.contains(&"-command"),
            "9.0-only -command leaked into 8.6: {in_86:?}",
        );
        let in_90 = regsub.switch_names(Some(DialectSet::TCL90));
        assert!(
            in_90.contains(&"-command"),
            "-command missing under 9.0: {in_90:?}",
        );
        // No filter → every declared option, no duplicates.
        let all = regsub.switch_names(None);
        assert!(all.contains(&"-command"));
        let mut dedup = all.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(dedup.len(), all.len(), "switch_names returned duplicates");
    }

    #[test]
    fn tcl9_commands_from_pr_433_are_registered() {
        let reg = CommandRegistry::build_default();
        for name in [
            "foreachLine",
            "readFile",
            "writeFile",
            "lpop",
            "const",
            "tcl::idna",
            "::tcl::idna",
            "tcl::process",
            "::tcl::process",
        ] {
            assert!(
                reg.get(name).is_some(),
                "{name} not registered after SYNC-MAY19-tcl9-commands",
            );
        }
    }

    #[test]
    fn coroinject_coroprobe_registered() {
        // Verify these two commands remain registered and visible to
        // the LSP.
        let reg = CommandRegistry::build_default();
        assert!(reg.get("coroinject").is_some());
        assert!(reg.get("coroprobe").is_some());
    }

    #[test]
    fn tcl9_commands_gated_to_tcl90() {
        use crate::dialects::DialectSet;
        let reg = CommandRegistry::build_default();
        for name in ["foreachLine", "readFile", "writeFile", "lpop"] {
            let spec = reg.get(name).expect("registered");
            // A 9.0 addition is available in 9.0 *and* 9.1 (a `.1` release is
            // additive — verified against C Tcl 9.1b0 doc/*.n), so it is gated
            // `TCL90_PLUS`, not `TCL90`-only.
            assert_eq!(
                spec.dialects,
                Some(DialectSet::TCL90_PLUS),
                "{name} should be Tcl 9.0+",
            );
            assert!(spec.supports_dialect(DialectSet::TCL90));
            assert!(spec.supports_dialect(DialectSet::TCL91));
            assert!(!spec.supports_dialect(DialectSet::TCL86));
        }
        // Unlike the four above, `const` is `dialects = None`
        // (universal) rather than Tcl-9.0-gated, so it is valid inside
        // iRules events and `commands_for_event` accepts it.
        assert_eq!(
            reg.get("const").expect("registered").dialects,
            None,
            "const should be universal (it is dialect-agnostic)",
        );
    }

    #[test]
    fn timerate_registered_with_body_and_int_hint() {
        // `timerate` measures the rate of execution of a script.
        use crate::arg_role::ArgRole;
        use crate::side_effects::SideEffectTarget;
        use crate::types::TclType;
        let reg = CommandRegistry::build_default();
        let spec = reg.get("timerate").expect("timerate registered");
        assert_eq!(spec.name, "timerate");
        // BODY role on arg 0, INT type hint (shimmers) on arg 1.
        assert_eq!(spec.arg_role_at(0), Some(ArgRole::Body));
        assert_eq!(
            spec.arg_types
                .iter()
                .find(|(i, _)| *i == 1)
                .map(|(_, h)| h.expected),
            Some(Some(TclType::Int)),
        );
        // Unbounded arity: at least the command word, no upper bound.
        assert!(!spec.arity.accepts(0));
        assert!(spec.arity.accepts(1));
        assert!(spec.arity.accepts(6));
        assert_eq!(spec.return_type, Some(TclType::String));
        // The body runs arbitrary code → an UNKNOWN-target read+write effect.
        assert!(
            spec.side_effects
                .iter()
                .any(|e| e.target == SideEffectTarget::Unknown && e.reads && e.writes),
            "timerate should declare an UNKNOWN read+write side effect",
        );
    }

    #[test]
    fn lookup_for_command() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get("for").unwrap();
        assert_eq!(spec.name, "for");
        assert!(spec.traits.contains(Traits::CONTROL_FLOW));
        assert!(spec.traits.contains(Traits::HAS_LOOP_BODY));
        assert_eq!(spec.arity, crate::arity::Arity::exact(4));
    }

    #[test]
    fn arg_roles_for_static_command() {
        let reg = CommandRegistry::build_default();
        let bodies =
            reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Body);
        assert!(bodies.contains(&0)); // init
        assert!(bodies.contains(&2)); // next
        assert!(bodies.contains(&3)); // body
        assert!(!bodies.contains(&1)); // cond is Expr, not Body
    }

    #[test]
    fn arg_roles_for_expr() {
        let reg = CommandRegistry::build_default();
        let exprs =
            reg.arg_indices_for_role("for", &["init", "cond", "next", "body"], ArgRole::Expr);
        assert_eq!(exprs, vec![1]); // only the condition
    }

    #[test]
    fn if_marks_structural_keywords() {
        let reg = CommandRegistry::build_default();
        let args = [
            "1", "then", "{a}", "elseif", "2", "then", "{b}", "else", "{c}",
        ];
        let kw = reg.arg_indices_for_role("if", &args, ArgRole::Keyword);
        // then@1, elseif@3, then@5, else@7
        assert_eq!(kw, vec![1, 3, 5, 7], "{kw:?}");
        // The bodies and exprs still resolve too.
        let bodies = reg.arg_indices_for_role("if", &args, ArgRole::Body);
        assert!(bodies.contains(&2) && bodies.contains(&6) && bodies.contains(&8));
    }

    #[test]
    fn try_marks_structural_keywords() {
        let reg = CommandRegistry::build_default();
        let args = ["{body}", "on", "error", "{e}", "{h}", "finally", "{f}"];
        let kw = reg.arg_indices_for_role("try", &args, ArgRole::Keyword);
        // on@1, finally@5
        assert_eq!(kw, vec![1, 5], "{kw:?}");
    }

    #[test]
    fn commands_with_trait_query() {
        let reg = CommandRegistry::build_default();
        let control_flow = reg.commands_with_trait(Traits::CONTROL_FLOW);
        assert!(control_flow.contains(&"for"));
        assert!(control_flow.contains(&"if"));
        assert!(control_flow.contains(&"while"));
        assert!(!control_flow.contains(&"puts"));
    }

    #[test]
    fn byte_compiled_covers_the_core_builtins() {
        let reg = CommandRegistry::build_default();
        // Every registered command the minifier must never alias.
        // The former built-in skip list (minus the
        // non-command keywords `else` / `elseif` and the unregistered
        // `pwd`).
        let expected = [
            "set",
            "unset",
            "proc",
            "if",
            "while",
            "for",
            "foreach",
            "switch",
            "return",
            "break",
            "continue",
            "expr",
            "catch",
            "try",
            "throw",
            "package",
            "namespace",
            "upvar",
            "uplevel",
            "variable",
            "global",
            "append",
            "lappend",
            "incr",
            "info",
            "string",
            "list",
            "llength",
            "lindex",
            "lrange",
            "lsort",
            "lsearch",
            "lreplace",
            "linsert",
            "dict",
            "array",
            "regexp",
            "regsub",
            "scan",
            "format",
            "open",
            "close",
            "read",
            "gets",
            "eof",
            "flush",
            "seek",
            "tell",
            "fconfigure",
            "fcopy",
            "fileevent",
            "socket",
            "after",
            "update",
            "vwait",
            "rename",
            "source",
            "eval",
            "apply",
            "tailcall",
            "error",
            "cd",
            "file",
            "glob",
            "clock",
            "binary",
            "encoding",
            "interp",
            "load",
            "exit",
            "pid",
            "exec",
            "chan",
            "puts",
        ];
        for name in expected {
            assert!(
                reg.is_byte_compiled(name),
                "{name} should carry Traits::BYTE_COMPILED"
            );
        }
        // A user-proc-like name and a command outside the curated
        // skip set must not carry the trait.
        assert!(!reg.is_byte_compiled("my_helper_proc"));
        assert!(!reg.is_byte_compiled("split"));
    }

    #[test]
    fn not_proc_factory_covers_registered_skip_heads() {
        let reg = CommandRegistry::build_default();
        // Registered heads from the former `_FACTORY_SKIP_HEADS` list
        // (the four non-command heads method / classmethod /
        // itcl::class / ::itcl::class are handled by the scanner's
        // residual set, not the registry).
        let expected = [
            "proc",
            "namespace",
            "if",
            "switch",
            "while",
            "for",
            "foreach",
            "try",
            "catch",
            "eval",
            "apply",
            "expr",
            "uplevel",
            "upvar",
            "variable",
            "set",
            "lappend",
            "dict",
            "array",
            "string",
            "list",
            "lindex",
            "package",
            "source",
            "interp",
            "oo::class",
            "oo::define",
            "oo::objdefine",
        ];
        for name in expected {
            assert!(
                reg.is_not_proc_factory(name),
                "{name} should carry Traits::NOT_PROC_FACTORY"
            );
        }
        // A real factory-wrapper head must not be skipped.
        assert!(!reg.is_not_proc_factory("my_factory"));
    }

    #[test]
    fn frameless_runtime_covers_the_audited_allow_list() {
        let reg = CommandRegistry::build_default();
        let got: std::collections::HashSet<&str> = reg
            .commands_with_trait(Traits::FRAMELESS_RUNTIME)
            .into_iter()
            .collect();
        let expected: std::collections::HashSet<&str> = [
            "list",
            "lindex",
            "lrange",
            "linsert",
            "llength",
            "lsort",
            "lsearch",
            "lappend",
            "lreverse",
            "lreplace",
            "lrepeat",
            "lassign",
            "concat",
            "split",
            "join",
            "string",
            "expr",
            "global",
            "variable",
            "upvar",
            "namespace",
            "set",
            "incr",
            "append",
            "unset",
            "puts",
            "return",
            "error",
            "continue",
            "break",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            got, expected,
            "FRAMELESS_RUNTIME stamps drifted from the audited allow-list"
        );
    }

    #[test]
    fn dialect_filter() {
        let reg = CommandRegistry::build_default();
        let spec = reg.get_for_dialect("dict", DialectSet::TCL86);
        assert!(spec.is_some());
        // dict is tcl8.5+ so should NOT be available in 8.4
        let spec84 = reg.get_for_dialect("dict", DialectSet::TCL84);
        assert!(spec84.is_none());
    }

    #[test]
    fn subcommand_arg_roles() {
        let reg = CommandRegistry::build_default();
        let bodies =
            reg.arg_indices_for_role("dict", &["for", "{k v}", "$d", "body"], ArgRole::Body);
        // dict for {varList} dictExpr body → body is at index 3 (subcmd=0, args 1-based +1)
        assert!(bodies.contains(&3));
    }

    #[test]
    fn variable_write_commands() {
        let reg = CommandRegistry::build_default();
        let set_vars = reg.arg_indices_for_role("set", &["x", "1"], ArgRole::VarWrite);
        assert_eq!(set_vars, vec![0]);
    }

    /// `trace add variable name ops body` declares arg 1
    /// (the variable name) as `VarWrite` via the registry.
    #[test]
    fn arg_indices_for_role_trace_add_variable_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["add", "variable", "x", "write", "body"],
            ArgRole::VarWrite,
        );
        // +1 for subcommand offset.  arg "x" is at idx 2 in the
        // full args list (sub args[1] + 1).
        assert!(writes.contains(&2), "VarWrite writes={writes:?}");
    }

    /// `trace add execution` does NOT declare `VarWrite`
    /// (the second arg is a command name, not a variable).
    #[test]
    fn arg_indices_for_role_trace_add_execution_no_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["add", "execution", "foo", "enter", "body"],
            ArgRole::VarWrite,
        );
        assert!(writes.is_empty(), "VarWrite writes={writes:?}");
    }

    /// `trace remove variable` behaves like `trace add variable`:
    /// both alias spellings flow through the same `VarWrite` query.
    #[test]
    fn arg_indices_for_role_trace_remove_variable_var_write() {
        let reg = CommandRegistry::build_default();
        let writes = reg.arg_indices_for_role(
            "trace",
            &["remove", "variable", "y", "write", "body"],
            ArgRole::VarWrite,
        );
        assert!(writes.contains(&2), "VarWrite writes={writes:?}");
    }

    /// `global` / `variable` / `upvar` carry
    /// `CREATES_DYNAMIC_BARRIER` so SSA's barrier-def walk knows the
    /// per-arg list belongs to `var_scoping`, not the role-driven
    /// `VarWrite` query.
    #[test]
    fn creates_dynamic_barrier_trait_marks_scope_aliases() {
        let reg = CommandRegistry::build_default();
        for cmd in &["global", "variable", "upvar"] {
            assert!(
                reg.get(cmd)
                    .unwrap()
                    .traits
                    .contains(Traits::CREATES_DYNAMIC_BARRIER),
                "{cmd} should carry CREATES_DYNAMIC_BARRIER",
            );
        }
        // `set` does NOT carry the trait — its VarWrite at arg 0 is
        // a single-target def, not a vararg list.
        assert!(
            !reg.get("set")
                .unwrap()
                .traits
                .contains(Traits::CREATES_DYNAMIC_BARRIER)
        );
    }

    /// `dict with` / `dict update` arg 0 (the dict variable) plays
    /// both `VarRead` and `VarWrite` roles. This is emitted via
    /// duplicate `(idx, role)` rows in the resolver (the multi-role
    /// observable behaviour is what consumers query).
    /// Every spec defaults to `BodyKind::Plain` unless it
    /// opts into `Structural`.
    #[test]
    fn body_kind_default_plain() {
        use crate::body_kind::BodyKind;
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("set").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("if").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("while").unwrap().body_kind, BodyKind::Plain);
        assert_eq!(reg.get("foreach").unwrap().body_kind, BodyKind::Plain);
    }

    /// `proc` / `oo::class` / `oo::define` / `oo::objdefine`
    /// stamp `Structural` so SSA skips their body args from the
    /// enclosing block's data flow.
    #[test]
    fn body_kind_structural_marks() {
        use crate::body_kind::BodyKind;
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("proc").unwrap().body_kind, BodyKind::Structural);
        assert_eq!(
            reg.get("oo::class").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("oo::define").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("oo::objdefine").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("snit::method").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("snit::typemethod").unwrap().body_kind,
            BodyKind::Structural
        );
        assert_eq!(
            reg.get("uri::register").unwrap().body_kind,
            BodyKind::Structural
        );
    }

    /// iRules `when` event handler bodies are structural.
    #[test]
    fn body_kind_irules_when_structural() {
        use crate::body_kind::BodyKind;
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        assert_eq!(reg.get("when").unwrap().body_kind, BodyKind::Structural);
    }

    /// `body_arg_implicit_args` defaults to 0 and is set on
    /// `fileutil::updateInPlace` (which appends file contents to
    /// the body's first command at runtime).
    #[test]
    fn body_arg_implicit_args_defaults_zero_except_fileutil_updateinplace() {
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.get("set").unwrap().body_arg_implicit_args, 0);
        assert_eq!(reg.get("proc").unwrap().body_arg_implicit_args, 0);
        assert_eq!(
            reg.get("fileutil::updateInPlace")
                .unwrap()
                .body_arg_implicit_args,
            1,
        );
    }

    #[test]
    fn arg_indices_for_role_dict_with_multirole() {
        let reg = CommandRegistry::build_default();
        let reads = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarRead);
        let writes = reg.arg_indices_for_role("dict", &["with", "$var", "body"], ArgRole::VarWrite);
        assert!(reads.contains(&1), "VarRead reads={reads:?}");
        assert!(writes.contains(&1), "VarWrite writes={writes:?}");
    }

    #[test]
    fn arg_indices_for_role_dict_update_multirole() {
        let reg = CommandRegistry::build_default();
        let reads = reg.arg_indices_for_role(
            "dict",
            &["update", "$var", "k", "vname", "body"],
            ArgRole::VarRead,
        );
        let writes = reg.arg_indices_for_role(
            "dict",
            &["update", "$var", "k", "vname", "body"],
            ArgRole::VarWrite,
        );
        assert!(reads.contains(&1), "VarRead reads={reads:?}");
        assert!(writes.contains(&1), "VarWrite writes={writes:?}");
    }

    #[test]
    fn dynamic_arg_role_resolution() {
        let reg = CommandRegistry::build_default();
        // if expr body elseif expr body else body
        let roles = reg.arg_indices_for_role(
            "if",
            &[
                "$x", "then", "body1", "elseif", "$y", "body2", "else", "body3",
            ],
            ArgRole::Body,
        );
        // Bodies are at positions 2, 5, 7
        assert!(roles.contains(&2));
        assert!(roles.contains(&5));
        assert!(roles.contains(&7));
    }

    #[test]
    fn load_irules_dialect() {
        let mut reg = CommandRegistry::build_default();
        assert!(reg.get("HTTP::header").is_none()); // not loaded yet
        reg.load_irules();
        assert!(reg.get("HTTP::header").is_some());
        assert!(reg.len() > 200); // should have 1000+ commands now
    }

    #[test]
    fn irules_command_has_irules_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let spec = reg.get("HTTP::header").unwrap();
        assert_eq!(spec.dialects, Some(DialectSet::IRULES));
    }

    #[test]
    fn irules_idempotent_load() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        let count1 = reg.len();
        reg.load_irules(); // second load should be no-op
        assert_eq!(reg.len(), count1);
    }

    #[test]
    fn default_includes_stdlib_and_tcllib() {
        let reg = CommandRegistry::build_default();
        // stdlib and tcllib are loaded by default
        assert!(reg.len() > 200);
    }

    #[test]
    fn load_tk_dialect() {
        // Tk is part of the always-known base registry now, so it is present
        // by default and a later `load_dialect(TK)` is an idempotent no-op.
        let reg = CommandRegistry::build_default();
        assert!(
            reg.get("grid").is_some(),
            "Tk commands are loaded by default"
        );
        let base_count = reg.len();
        let mut reg2 = CommandRegistry::build_default();
        reg2.load_dialect(DialectSet::TK);
        assert_eq!(
            reg2.len(),
            base_count,
            "load_dialect(TK) is a no-op after default load"
        );
    }

    #[test]
    fn load_iapps_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::IAPPS);
        assert!(reg.len() > 100);
    }

    #[test]
    fn load_expect_dialect() {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::EXPECT);
        assert!(reg.get("expect").is_some() || reg.get("spawn").is_some());
    }

    #[test]
    fn load_eda_synopsys() {
        let mut reg = CommandRegistry::build_default();
        reg.load_dialect(DialectSet::SYNOPSYS);
        assert!(reg.len() > 100);
    }

    #[test]
    fn resolve_call_unknown_command_returns_none() {
        let reg = CommandRegistry::build_default();
        assert!(
            reg.resolve_call("no_such_cmd", &[], DialectSet::empty())
                .is_none()
        );
    }

    #[test]
    fn resolve_call_top_level_command() {
        let reg = CommandRegistry::build_default();
        let resolved = reg
            .resolve_call("set", &["x", "1"], DialectSet::empty())
            .unwrap();
        assert_eq!(resolved.spec.name, "set");
        assert!(resolved.sub.is_none());
    }

    #[test]
    fn resolve_call_subcommand() {
        let reg = CommandRegistry::build_default();
        let resolved = reg
            .resolve_call("dict", &["create", "k", "v"], DialectSet::TCL86)
            .unwrap();
        assert_eq!(resolved.spec.name, "dict");
        let sub = resolved.sub.expect("dict create resolves to a subcommand");
        assert_eq!(sub.name, "create");
    }

    #[test]
    fn resolve_call_dialect_filter_blocks_tcl84_dict() {
        let reg = CommandRegistry::build_default();
        // dict is tcl8.5+; resolving against tcl8.4 must fail.
        assert!(
            reg.resolve_call("dict", &["create"], DialectSet::TCL84)
                .is_none()
        );
    }

    #[test]
    fn resolve_call_picks_arity_matched_command_form_for_incr() {
        let reg = CommandRegistry::build_default();
        // `incr counter` — arity 1 → matches the implicit form.
        let r1 = reg
            .resolve_call("incr", &["counter"], DialectSet::empty())
            .unwrap();
        let f1 = r1.form.expect("incr should match a CommandForm");
        assert_eq!(f1.name, "implicit");
        assert_eq!(f1.arity, crate::arity::Arity::exact(1));

        // `incr counter 5` — arity 2 → matches the explicit form.
        let r2 = reg
            .resolve_call("incr", &["counter", "5"], DialectSet::empty())
            .unwrap();
        let f2 = r2.form.expect("incr counter 5 should match a CommandForm");
        assert_eq!(f2.name, "explicit");
        assert_eq!(f2.arity, crate::arity::Arity::exact(2));
    }

    #[test]
    fn resolve_call_picks_arity_matched_command_form_for_lset() {
        let reg = CommandRegistry::build_default();
        // `lset lst value` — arity 2 → replace form.
        let replace = reg
            .resolve_call("lset", &["lst", "value"], DialectSet::TCL86)
            .unwrap();
        assert_eq!(replace.form.unwrap().name, "replace");

        // `lset lst 0 value` — arity 3 → single_index form.
        let single = reg
            .resolve_call("lset", &["lst", "0", "value"], DialectSet::TCL86)
            .unwrap();
        assert_eq!(single.form.unwrap().name, "single_index");

        // `lset lst 0 1 2 value` — arity 5 → flat_path form.
        let flat = reg
            .resolve_call("lset", &["lst", "0", "1", "2", "value"], DialectSet::TCL86)
            .unwrap();
        assert_eq!(flat.form.unwrap().name, "flat_path");
    }

    // -- ``resolve_option_terminator`` (W304 driver)
    //
    // Exercises option-terminator resolution for each command.
    // Each W304 fixture
    // is rooted in one of these resolver outcomes; the resolver tests
    // here pin the per-command shape, the analyser tests pin the
    // tristate-severity / two-diagnostic / code-fix behaviour.

    #[test]
    fn resolve_option_terminator_returns_none_for_unknown_command() {
        let reg = CommandRegistry::build_default();
        assert!(
            reg.resolve_option_terminator("unknownthing", &[], DialectSet::empty())
                .is_none()
        );
    }

    #[test]
    fn resolve_option_terminator_returns_none_for_command_without_terminator() {
        let reg = CommandRegistry::build_default();
        // ``set`` does not declare a ``--`` terminator option.
        assert!(
            reg.resolve_option_terminator("set", &["x", "1"], DialectSet::empty())
                .is_none()
        );
    }

    #[test]
    fn resolve_option_terminator_form_level_for_regexp() {
        let reg = CommandRegistry::build_default();
        let profile = reg
            .resolve_option_terminator("regexp", &[], DialectSet::empty())
            .expect("regexp declares -- at the form level");
        assert_eq!(profile.scan_start, 0);
        assert!(profile.subcommand.is_none());
        assert!(profile.warn_without_terminator);
        // ``-start`` takes a value; ``-nocase`` does not.
        // ``-start`` takes a value; ``-nocase`` does not.  The
        // resolver returns the borrowed options slice; callers
        // filter via ``OptionSpec::takes_value``.
        assert!(
            profile
                .options
                .iter()
                .any(|o| o.name == "-start" && o.takes_value)
        );
        assert!(
            profile
                .options
                .iter()
                .any(|o| o.name == "-nocase" && !o.takes_value)
        );
    }

    #[test]
    fn resolve_option_terminator_subcommand_scoped_for_file_delete() {
        let reg = CommandRegistry::build_default();
        let profile = reg
            .resolve_option_terminator("file", &["delete", "$path"], DialectSet::empty())
            .expect("file delete declares -- at the subcommand level");
        assert_eq!(profile.scan_start, 1);
        assert_eq!(profile.subcommand, Some("delete"));
    }

    #[test]
    fn resolve_option_terminator_subcommand_without_terminator_returns_none() {
        let reg = CommandRegistry::build_default();
        // ``file mtime`` has no ``--`` terminator.
        let profile = reg.resolve_option_terminator("file", &["mtime", "$p"], DialectSet::empty());
        assert!(profile.is_none(), "got {profile:?}");
    }

    #[test]
    fn resolve_option_terminator_warn_flag_off_for_non_regexp() {
        let reg = CommandRegistry::build_default();
        // ``unset`` declares ``--`` but does not carry the
        // ``WARN_WITHOUT_TERMINATOR`` trait — only ``regexp`` does.
        let profile = reg
            .resolve_option_terminator("unset", &["$x"], DialectSet::empty())
            .expect("unset declares --");
        assert!(!profile.warn_without_terminator);
    }

    // -- ``is_canonical_list_command`` (W101 safe-idiom driver)

    #[test]
    fn is_canonical_list_command_includes_list_and_split_excludes_concat() {
        let reg = CommandRegistry::build_default();
        assert!(reg.is_canonical_list_command("list"));
        assert!(reg.is_canonical_list_command("linsert"));
        assert!(reg.is_canonical_list_command("split"));
        assert!(reg.is_canonical_list_command("lreverse"));
        // ``concat`` returns LIST but is the explicit non-canonical
        // exclusion.
        assert!(!reg.is_canonical_list_command("concat"));
        // Non-list commands (e.g. ``set``) are filtered out.
        assert!(!reg.is_canonical_list_command("set"));
        // Unknown commands return false.
        assert!(!reg.is_canonical_list_command("unknownthing"));
    }

    #[test]
    fn is_canonical_list_command_handles_compound_subcommand_keys() {
        let reg = CommandRegistry::build_default();
        // ``dict keys`` returns LIST.
        assert!(reg.is_canonical_list_command("dict keys"));
        // ``dict get`` returns String (or unspecified) — not canonical.
        assert!(!reg.is_canonical_list_command("dict froob"));
    }

    #[test]
    fn irules_sink_commands_carry_structural_options() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();

        let respond = reg.get("HTTP::respond").expect("HTTP::respond loaded");
        let opts: Vec<&str> = respond.options.iter().map(|o| o.name).collect();
        // Option set follows the reference standard:
        // -version/-content/-ifile/
        // -noserver/-reset. (`-status` is the positional status arg.)
        assert!(
            opts.contains(&"-version")
                && opts.contains(&"-content")
                && opts.contains(&"-noserver")
                && opts.contains(&"-reset"),
            "HTTP::respond options {opts:?} should include -version / -content / -noserver / -reset",
        );
        let noserver = respond
            .options
            .iter()
            .find(|o| o.name == "-noserver")
            .unwrap();
        assert!(!noserver.takes_value);
        let version = respond
            .options
            .iter()
            .find(|o| o.name == "-version")
            .unwrap();
        assert!(version.takes_value);

        let header = reg.get("HTTP::header").expect("HTTP::header loaded");
        let header_opts: Vec<&str> = header.options.iter().map(|o| o.name).collect();
        assert!(
            header_opts.contains(&"-noupdate"),
            "HTTP::header options {header_opts:?} should include -noupdate",
        );
    }

    #[test]
    fn xc_translatability_helpers_read_spec_flags() {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();

        // `xc_translatable: Some(false)` → never translatable (consumed by the
        // `f5-xc` translator's XC300 branch).
        assert!(reg.is_xc_never_translatable("eval"));
        assert!(!reg.is_xc_translatable_override("eval"));

        // `xc_translatable: Some(true)` → translatable override despite an
        // otherwise-untranslatable namespace prefix (e.g. `IP::`, `ASM::`).
        assert!(reg.is_xc_translatable_override("IP::client_addr"));
        assert!(!reg.is_xc_never_translatable("IP::client_addr"));

        // Commands with no `xc_translatable` flag report neither.
        assert!(!reg.is_xc_never_translatable("set"));
        assert!(!reg.is_xc_translatable_override("set"));
        // An unknown command name is safely neither.
        assert!(!reg.is_xc_never_translatable("no_such_command_xyz"));
        assert!(!reg.is_xc_translatable_override("no_such_command_xyz"));
    }
}
