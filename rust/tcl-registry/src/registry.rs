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

use crate::arg_role::{AppendedArity, ArgRole};
use crate::arity::Arity;
use crate::body_kind::BodyKind;
use crate::command_table::CommandTableEffect;
use crate::dialects::DialectSet;
use crate::forms::CommandForm;
use crate::hooks::{AnalyserHookId, CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::spec::{BytePayloadSpec, CommandSpec, SubCommand};
use crate::traits::Traits;
use crate::types::VarWriteTyping;

/// The trait union defining a **frame-sensitive** command — see
/// [`CommandRegistry::is_frame_sensitive`].
const FRAME_SENSITIVE_TRAITS: Traits = Traits::TERMINATES_BLOCK
    .union(Traits::TRANSFERS_CONTROL)
    .union(Traits::CREATES_SCOPE_ALIAS)
    .union(Traits::CREATES_BARRIER);

/// Resolved metadata for an iRules event — the result of
/// [`CommandRegistry::event_info`].
#[derive(Debug, Clone)]
pub struct EventInfo {
    /// The upper-cased event name as queried.
    pub event: String,
    /// The BIG-IP release this event is declared present from — explicit
    /// data, or the axis baseline (15.0.0) for a known event with none.
    /// `None` only for unknown events.
    pub bigip_min_version: Option<&'static str>,
    /// The last BIG-IP release providing this event; `None` = still
    /// present (the open maximum).
    pub bigip_max_version: Option<&'static str>,
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
    /// The dialect profile this registry was built for, when it came from
    /// `registry_for_profile` / `registry_for_dialect`. `None` for
    /// hand-assembled registries (tests, ad-hoc tools), which fall back to
    /// the `loaded_dialects`-derived behaviour answers.
    profile: Option<&'static tcl_dialect::DialectProfile>,
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
                // Normalise away a leading `::` so a spec registered only in
                // its fully-qualified spelling (e.g.
                // `::tcl::unsupported::corotype`, which has no separate bare
                // registration) still matches `known_in_any_dialect`'s
                // already-bare query — the caller strips a literal `::` head
                // from the source text before calling in, so the set must be
                // bare-normalised too or the two never agree.
                set.insert(spec.name.strip_prefix("::").unwrap_or(spec.name));
            }
        };
        add(crate::commands::bpf::bpf_command_specs());
        add(crate::commands::tcl::tcl_command_specs());
        add(crate::commands::stdlib::stdlib_command_specs());
        add(crate::commands::tcllib::tcllib_command_specs());
        add(crate::commands::argparse::argparse_command_specs());
        add(crate::commands::ticklecharts::ticklecharts_command_specs());
        add(crate::commands::itcl::itcl_command_specs());
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

/// Append the `args` indices consumed by value-taking options whose value role
/// (primary or secondary) equals `role`.
///
/// Walks `args` from `scan_start` (1 to skip a subcommand word, else 0),
/// matching option names/aliases literally — so `-1`, `$x`, `[cmd]` are treated
/// as positionals, never flags — and advancing past each recognised option and
/// the value word(s) it consumes ([`OptionSpec::value_indices`], which honours
/// arity and the `--` terminator). The emitted indices are absolute into `args`,
/// exactly like the positional roles, so consumers map them via `argv[idx + 1]`
/// unchanged. A two-way binding (`role: VarWrite, also_role: VarRead`) emits its
/// index for a query of either role — the multi-role convention, split across
/// queries.
fn push_option_value_roles(
    out: &mut Vec<usize>,
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
    role: ArgRole,
) {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = options.iter().find(|o| o.matches(args[i])) {
            let vals = opt.value_indices(args, i);
            if opt.value_role() == Some(role) || opt.value_also_role() == Some(role) {
                out.extend(vals.iter().copied());
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
}

/// Collect option values whose role is [`ArgRole::CommandPrefix`], paired with
/// the option's [`AppendedArity`] — the option-side companion to
/// [`push_option_value_roles`], used by [`CommandRegistry::command_prefixes`].
fn push_command_prefix_options(
    out: &mut Vec<(usize, AppendedArity)>,
    options: &[crate::hover::OptionSpec],
    args: &[&str],
    scan_start: usize,
) {
    let mut i = scan_start;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if let Some(opt) = options.iter().find(|o| o.matches(args[i])) {
            let vals = opt.value_indices(args, i);
            if opt.value_role() == Some(ArgRole::CommandPrefix) {
                let arity = opt.value_appended_arity();
                out.extend(vals.iter().map(|&v| (v, arity)));
            }
            i += 1 + vals.len();
        } else {
            i += 1;
        }
    }
}

impl CommandRegistry {
    /// Build the default registry with core Tcl + stdlib + tcllib commands.
    #[must_use]
    pub fn build_default() -> Self {
        let mut registry = Self {
            by_name: FxHashMap::default(),
            loaded_dialects: DialectSet::empty(),
            profile: None,
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
        for spec in crate::commands::ticklecharts::ticklecharts_command_specs() {
            registry.insert(spec);
        }
        for spec in crate::commands::itcl::itcl_command_specs() {
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
            // The tmsh shell's own pack: the `tmsh::` surface shared with
            // iApps (tagged `IAPPS|TMSH`), without the iApp-only commands
            // (Milestone 6, D8).
            d if d == DialectSet::TMSH => crate::commands::iapps::tmsh_command_specs(),
            d if d == DialectSet::TK => crate::commands::tk::tk_command_specs(),
            d if d == DialectSet::EXPECT => crate::commands::expect::expect_command_specs(),
            // The EDA shells load by profile identity via `load_eda_packs`
            // (below), not a DialectSet bit — they are modelled as base-Tcl-
            // version dialects plus `required_package`-gated command libraries
            // (design doc `eda-library-packages.md`).
            _ => Vec::new(),
        };
        for spec in specs {
            self.insert(spec);
        }
        self.loaded_dialects |= dialect;
    }

    /// Load an EDA shell profile's command packs by profile name — the shared
    /// `sdc_base` constraint/collection library plus the vendor's tool packs.
    ///
    /// EDA shells are modelled as a base Tcl version (loaded via
    /// [`Self::load_dialect`] with the version bit) plus `required_package`-
    /// gated libraries, rather than a vendor `DialectSet` bit (design doc
    /// `eda-library-packages.md`), so their packs load by profile identity.
    /// A no-op for any non-EDA profile name.
    pub fn load_eda_packs(&mut self, profile_name: &str) {
        let vendor = match profile_name {
            "xilinx-eda-tcl" => crate::commands::eda_xilinx::eda_xilinx_command_specs(),
            "synopsys-eda-tcl" => crate::commands::eda_synopsys::eda_synopsys_command_specs(),
            "cadence-eda-tcl" => crate::commands::eda_cadence::eda_cadence_command_specs(),
            "intel-quartus-eda-tcl" => crate::commands::eda_quartus::eda_quartus_command_specs(),
            "mentor-eda-tcl" => crate::commands::eda_mentor::eda_mentor_command_specs(),
            _ => return,
        };
        for spec in crate::commands::sdc_base::sdc_base_command_specs() {
            self.insert(spec);
        }
        for spec in vendor {
            self.insert(spec);
        }
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
    /// Tcl 9.0 dropped the leading-zero octal rule (TIP 114): `08` parses as
    /// decimal 8 and `010` as decimal 10. Every earlier Tcl (8.4/8.5/8.6) and
    /// every 8.x-derived dialect (f5-irules ≈ 8.4, f5-iapps ≈ 8.5/8.6, the EDA
    /// dialects) keeps the octal rule, where `08`/`09` are *invalid* octal
    /// (treated as a string in `==`/`!=`) and `010` is 8.
    ///
    /// TIP 114 lands in tcl9.0 and stays in tcl9.1 (and any later 9.x), so the
    /// decimal rule applies to *every* Tcl 9 dialect, not tcl9.0 alone. The
    /// per-dialect registry built by `registry_for_dialect` records its Tcl
    /// version via [`Self::load_dialect`], so a registry whose `loaded_dialects`
    /// intersects [`DialectSet::TCL90_PLUS`] (tcl9.0, tcl9.1, …) is decimal;
    /// every other dialect (8.4/8.5/8.6 and the F5/EDA registries, which never
    /// load a Tcl-9 version bit) reads leading zeros as octal.
    #[must_use]
    pub fn leading_zero_is_octal(&self) -> bool {
        self.octal_fold_policy().unwrap_or(true)
    }

    /// The three-valued leading-zero fold policy for this registry's
    /// dialect. A profile-built registry (`registry_for_profile` /
    /// `registry_for_dialect`) answers from the profile's runtime base:
    /// `Some(true)` = 8.x octal, `Some(false)` = 9.x decimal (`bpf`
    /// included, D7), `None` = abstain — no Tcl runtime to have an opinion
    /// (`f5-bigip`, the unknown-dialect fallback; §11.1 of the
    /// dialect-profile model). A hand-assembled registry keeps the
    /// historical `loaded_dialects` derivation (a version pack records its
    /// bit, so a 9.x load reads decimal).
    #[must_use]
    pub fn octal_fold_policy(&self) -> Option<bool> {
        match self.profile {
            Some(p) => p.leading_zero_is_octal.as_bool(),
            None => Some(!self.loaded_dialects.intersects(DialectSet::TCL90_PLUS)),
        }
    }

    /// Stamp the dialect profile this registry serves. Called by the
    /// per-profile cache (`registry_for_profile`) so behaviour queries
    /// (`octal_policy`, future runtime projections) answer from the
    /// profile rather than re-deriving from loaded packs.
    pub(crate) fn set_profile(&mut self, profile: &'static tcl_dialect::DialectProfile) {
        self.profile = Some(profile);
    }

    /// The dialect profile this registry was built for, if any.
    #[must_use]
    pub fn profile(&self) -> Option<&'static tcl_dialect::DialectProfile> {
        self.profile
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

    /// The typed BPF-Tcl lowering descriptor for `name`, when `name` is a
    /// BPF-dialect command (see [`crate::bpf_op`]).  The BPF-Tcl front-end
    /// dispatches on this — never on the command name.
    #[must_use]
    pub fn bpf_op(&self, name: &str) -> Option<&'static crate::bpf_op::BpfOpSpec> {
        self.get(name).and_then(|s| s.bpf_op)
    }

    /// Look up a command spec filtered by dialect, picking the
    /// **most-specific** visible spec (`best_visible` — §5.3's single
    /// selection rule).
    ///
    /// As with [`Self::get`], a leading `::` falls back to the bare name.
    ///
    /// A registry built for a dialect profile additionally applies that
    /// profile's SUBTRACTIVE rules ([`Self::spec_visible`]) whenever the
    /// queried mask concerns the profile's own availability — so a bare
    /// `IRULES` mask query on the f5-irules registry can never re-admit a
    /// banned command, no matter which consumer asks
    /// (dialect-profile-model.md §9.2).
    #[must_use]
    pub fn get_for_dialect(&self, name: &str, dialect: DialectSet) -> Option<&CommandSpec> {
        self.by_name
            .get(name)
            .or_else(|| {
                name.strip_prefix("::")
                    .and_then(|bare| self.by_name.get(bare))
            })
            .and_then(|specs| self.best_visible(specs, dialect))
    }

    /// Resolve `head` the way this registry's own availability rules
    /// resolve it: through [`Self::get_for_dialect`] against the dialect
    /// profile this registry was built for, or through the dialect-agnostic
    /// [`Self::get`] when it has no profile.
    ///
    /// The single place the "same availability rules as `get`" promise made
    /// by the profile-aware behaviour queries is implemented, so a query
    /// added later cannot quietly answer for a command the profile's
    /// dialect does not have.
    fn spec_for_this_registry(&self, head: &str) -> Option<&CommandSpec> {
        match self.profile {
            Some(profile) => self.get_for_dialect(head, profile.availability_mask),
            None => self.get(head),
        }
    }

    /// Which `TclOO` method-context keyword `head` is, if it is one.
    ///
    /// The registry-first replacement for the `head == "my"` /
    /// `matches!(head, "my" | "next" | "nextto")` literals consumers used to
    /// carry (issue #1050). Every consumer that needs "is this word a method
    /// dispatch or introspection keyword, and which kind" asks here, so a
    /// dialect that gains or loses one of them propagates through the specs
    /// rather than through a walker edit.
    ///
    /// A leading `::` resolves to the bare name (consumers previously
    /// matched `"my" | "::my"` by hand), matching [`Self::get`].
    ///
    /// Dialect-aware through the registry instance itself: a registry built
    /// by `registry_for_dialect` / `registry_for_profile` answers under that
    /// profile's availability mask, so all four keywords — every one of them
    /// `TCL86_PLUS` — return `None` from a `tcl8.4` or `tcl8.5` registry. A
    /// profile-less registry (`CommandRegistry::build_default`) answers
    /// dialect-agnostically, exactly as [`Self::get`] does.
    ///
    /// `link` is **not** a keyword here and must not be added: it creates
    /// per-class bareword commands rather than dispatching one (issue
    /// #1026). Nor is `self`'s definer-grammar homonym — the `self` word
    /// inside an `oo::define` body is a member-grammar wrapper, resolved
    /// through [`crate::definer`], not a command head.
    #[must_use]
    pub fn method_dispatch_keyword(&self, head: &str) -> Option<MethodDispatchKind> {
        let traits = self.spec_for_this_registry(head)?.traits;
        if traits.contains(Traits::TCLOO_SELF_DISPATCH) {
            Some(MethodDispatchKind::SelfDispatch)
        } else if traits.contains(Traits::TCLOO_NEXT_CHAIN) {
            Some(MethodDispatchKind::NextChain)
        } else if traits.contains(Traits::TCLOO_INTROSPECTION) {
            Some(MethodDispatchKind::Introspection)
        } else {
            None
        }
    }

    /// Whether `head`'s **bare** spelling resolves only from inside a
    /// `TclOO` method context — the registry-side half of issue #1026's
    /// scoping rule; see [`Traits::TCLOO_METHOD_CONTEXT`] for the oracle
    /// transcripts.
    ///
    /// Consumers pair this with their own "is this call site inside a
    /// `TclOO` method body?" fact
    /// (`tcl_compiler::analyser::scope::innermost_scope_reaches_oo_helpers`)
    /// — the registry knows *which* commands are scoped, the call site
    /// knows *where* it is, and neither needs the other's command names.
    ///
    /// Dialect-aware exactly like [`Self::method_dispatch_keyword`]: a
    /// registry built for a profile answers under that profile's
    /// availability mask, so a `tcl8.4` registry — which has no `TclOO` at
    /// all — answers `false` for every one of them.
    ///
    /// A qualified spelling (`oo::Helpers::link`, `::oo::Helpers::link`) is
    /// a separate, unscoped spec, so it answers `false`: the command really
    /// does exist globally, and calling it outside a method is a runtime
    /// error rather than an unknown command.
    #[must_use]
    pub fn resolves_only_in_method_context(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_METHOD_CONTEXT))
    }

    /// Whether **calling** `head` needs a real `TclOO` method invocation,
    /// not merely a frame that can resolve it — see
    /// [`Traits::TCLOO_REQUIRES_METHOD_FRAME`] for the oracle transcript.
    ///
    /// The narrower companion to [`Self::resolves_only_in_method_context`],
    /// and the two answer differently in exactly one place: a Tcl 9 class
    /// `initialise` / `initialize` body, where the whole family resolves
    /// (so `W123` must stay silent) but only `my` actually runs (so
    /// completion and hover must offer only `my`).
    ///
    /// A consumer deciding "may I offer this word here / does it hover"
    /// wants **both**: the word must resolve at the call site *and*, when
    /// this answers `true`, the call site must be a method frame rather
    /// than a bare object frame. `tcl_lsp_core::oo_dispatch` pairs them
    /// once so no consumer re-derives the rule.
    ///
    /// Dialect-aware exactly like [`Self::resolves_only_in_method_context`].
    #[must_use]
    pub fn requires_oo_method_frame(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_REQUIRES_METHOD_FRAME))
    }

    /// Whether `head` binds bareword aliases for methods of the current
    /// object — `TclOO`'s `link`; see [`Traits::TCLOO_BINDS_METHOD_ALIAS`].
    ///
    /// The registry-first replacement for the `texts[0] == "link"` literal
    /// the analyser's class-body walk used to carry (issue #1026), and
    /// dialect-aware for the same reason [`Self::method_dispatch_keyword`]
    /// is: `link` is 9.0-core / 8.6-via-`ooutil`, so an 8.5 registry answers
    /// `false`.
    #[must_use]
    pub fn binds_method_alias(&self, head: &str) -> bool {
        self.spec_for_this_registry(head)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_BINDS_METHOD_ALIAS))
    }

    /// The single spec-selection rule (§5.3, D6): among the specs of one
    /// name visible under `dialect`, pick the **most specific** — a
    /// dialect-scoped spec beats a catch-all (`dialects: None`), a tighter
    /// scope (fewer mask bits) beats a wider one, and among equals the
    /// *last-registered* spec wins, so curated pack overrides keep beating
    /// the data they shadow. `get_for_dialect`, the iRules event
    /// cross-product, and (via `ProfileQueries::resolve_command`) the CLI
    /// snapshot all resolve through this one rule.
    fn best_visible<'a>(
        &self,
        specs: &'a [CommandSpec],
        dialect: DialectSet,
    ) -> Option<&'a CommandSpec> {
        specs
            .iter()
            .enumerate()
            .filter(|(_, s)| self.spec_visible(s, dialect))
            .max_by_key(|&(index, s)| {
                let scope_tightness =
                    std::cmp::Reverse(s.dialects.map_or(u32::MAX, |d| d.bits().count_ones()));
                (s.dialects.is_some(), scope_tightness, index)
            })
            .map(|(_, s)| s)
    }

    /// The full availability test for a mask query on this registry: the
    /// spec's own dialect gate, plus — when this registry was built for a
    /// profile and the query concerns that profile's availability — the
    /// profile's subtractive disable list (§9) and, for a profile whose
    /// operators are not command heads, the operator-command exclusion.
    ///
    /// Public because generators projecting a command surface for an
    /// explicit mask (the Zed highlight queries project the profile's
    /// `grammar_union`, not its `availability_mask`) need the same
    /// subtractive semantics `get_for_dialect` applies internally.
    #[must_use]
    pub fn spec_visible(&self, spec: &CommandSpec, dialect: DialectSet) -> bool {
        if !spec.supports_dialect(dialect) {
            return false;
        }
        let Some(profile) = self.profile else {
            return true;
        };
        if !dialect.intersects(profile.availability_mask) {
            // The query is about some other dialect's availability; this
            // profile's operator-exclusion does not apply to it.
            return true;
        }
        // iRules availability is fully explicit in each spec's `dialects`
        // now (a command carries the `IRULES` bit iff iRules enables it), so
        // there is no subtractive ban list — the only remaining profile-level
        // exclusion is the operator-command one (math operators are not
        // command heads under iRules).
        profile.operators_as_commands
            || !spec
                .traits
                .contains(crate::traits::Traits::OPERATOR_COMMAND)
    }

    /// Return all registered command names.
    pub fn command_names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(String::as_str)
    }

    /// The [`ObjectClassSpec`] for a `TclOO` / megawidget class named
    /// `class_name`, or `None` when it is not a registry-modelled class.
    ///
    /// For a `TclOO` class the class name is the factory command name
    /// (`oo::class create Foo` binds command `Foo`), so this resolves through
    /// the ordinary command table — no separate index.  A leading `::` falls
    /// back to the bare name, as with [`Self::get`].
    #[must_use]
    pub fn object_class(&self, class_name: &str) -> Option<&crate::spec::ObjectClassSpec> {
        self.get(class_name).and_then(|s| s.object_class)
    }

    /// Resolve an instance method `method` on class `class_name`, walking
    /// declared superclasses breadth-first.  Returns the owning class's
    /// [`SubCommand`] method spec, or `None` when unresolved.
    #[must_use]
    pub fn instance_method(
        &self,
        class_name: &str,
        method: &str,
    ) -> Option<&crate::spec::SubCommand> {
        // FIFO so the walk is genuinely breadth-first and visits siblings in
        // declaration order — `Vec::pop` would make this a reversed-sibling
        // depth-first search, contradicting the documented contract.
        let mut queue = std::collections::VecDeque::from([class_name.to_string()]);
        let mut seen = std::collections::HashSet::new();
        while let Some(cls) = queue.pop_front() {
            if !seen.insert(cls.clone()) {
                continue;
            }
            let Some(class_spec) = self.object_class(&cls) else {
                continue;
            };
            if let Some(m) = class_spec.instance_method(method) {
                return Some(m);
            }
            for sup in class_spec.superclasses {
                queue.push_back((*sup).to_string());
            }
        }
        None
    }

    /// Resolve the [`ArgRole::CommandPrefix`] positions and appended arities for
    /// an instance-method dispatch `$obj method method_args…`.
    ///
    /// `method_args` are the words *after* the method name.  Mirrors the
    /// subcommand arm of [`Self::command_prefixes`] (static
    /// [`SubCommand::command_prefixes`] table ∪ `command_prefix_resolver` ∪
    /// command-prefix options), but keyed on the object's class + method rather
    /// than a top-level command — so `$g walk … -command cb` (option value) and
    /// `$t walkproc … cb` (trailing positional, resolver) light up the same
    /// references / call-graph / W123 / arity substrate.  Returned indices are
    /// relative to `method_args` (0 = first word after the method name).
    ///
    /// [`SubCommand::command_prefixes`]: crate::spec::SubCommand::command_prefixes
    #[must_use]
    pub fn instance_method_command_prefixes(
        &self,
        class_name: &str,
        method: &str,
        method_args: &[&str],
    ) -> Vec<(usize, AppendedArity)> {
        let Some(m) = self.instance_method(class_name, method) else {
            return Vec::new();
        };
        let n = method_args.len();
        let mut out: Vec<(usize, AppendedArity)> = Vec::new();
        if let Some(resolver) = m.command_prefix_resolver {
            out.extend(
                resolver(method_args)
                    .into_iter()
                    .map(|(i, a)| (i as usize, a)),
            );
        } else {
            out.extend(m.command_prefixes.iter().map(|(i, a)| (*i as usize, *a)));
        }
        push_command_prefix_options(&mut out, m.options, method_args, 0);
        out.retain(|&(idx, _)| idx < n);
        out
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
    /// This is the raw data view — resolution (which spec *wins* under a
    /// dialect) goes through [`Self::get_for_dialect`]'s most-specific
    /// rule.
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
        bigip_version: Option<&str>,
    ) -> Vec<&'a str> {
        let Some(props) = events.get_props(event) else {
            return Vec::new();
        };
        // The declared version range for an F5-surface spec: explicit
        // introduction/removal data, or the axis baseline (15.0) for a
        // spec with none. `bigip_version: None` keeps the pre-version
        // behaviour (no filtering) so digest-stable callers opt in.
        let version_ok = |spec: &CommandSpec| {
            let Some(version) = bigip_version else {
                return true;
            };
            let min = spec
                .min_version
                .or(tcl_dialect::VersionKey::BigipVersion.baseline_version());
            crate::version::within_range(version, min, spec.max_version)
        };
        let mut names: Vec<&str> = self
            .by_name
            .iter()
            .filter_map(|(name, specs)| {
                // Best spec for the dialect — the §5.3 most-specific rule,
                // matching `get_for_dialect`.
                let spec = self.best_visible(specs, DialectSet::IRULES)?;
                if !version_ok(spec) {
                    return None;
                }
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
        bigip_version: Option<&str>,
    ) -> EventInfo {
        let name = event.trim().to_uppercase();
        let target = bigip_version
            .or(tcl_dialect::VersionKey::BigipVersion.default_version())
            .unwrap_or("16.1.0");
        let known =
            !name.is_empty() && events.is_known(&name) && events.event_available_at(&name, target);
        let valid_commands: Vec<String> = if known {
            self.valid_irules_commands_for_event(&name, events, profiles, Some(target))
                .into_iter()
                .map(ToOwned::to_owned)
                .collect()
        } else {
            Vec::new()
        };
        let props = events.get_props(&name);
        let (bigip_min_version, bigip_max_version) = events
            .event_version_range(&name)
            .map_or((None, None), |(min, max)| (min, max));
        EventInfo {
            bigip_min_version,
            bigip_max_version,
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

    /// The symbol-definer descriptor for `name` in `dialect`, if the command
    /// binds a navigable definition name (a `tcltest::test` case, …).
    ///
    /// A leading `::` falls back to the bare name, as with [`Self::get`].  The
    /// analyser and signature scanner consult this to record outline symbols
    /// generically — the argument index and outline category come from the
    /// [`crate::symbol_def::SymbolDef`], never from a command-name check.
    #[must_use]
    pub fn defines_symbol(
        &self,
        name: &str,
        dialect: DialectSet,
    ) -> Option<&crate::symbol_def::SymbolDef> {
        self.get_for_dialect(name, dialect)
            .and_then(|s| s.defines_symbol.as_ref())
    }

    /// Every command name that declares a [`crate::symbol_def::SymbolDef`] in
    /// any registered spec, for consumers (the signature scanner) that
    /// precompute a symbol-definer lookup set rather than querying per-call.
    #[must_use]
    pub fn commands_defining_symbols(&self) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .iter()
                    .any(|s| s.defines_symbol.is_some())
                    .then_some(name.as_str())
            })
            .collect()
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

    /// Whether `name` **writes or modifies** the variable named by its
    /// first argument (`set` / `append` / `lappend` / `incr` / `lset`):
    /// [`Traits::FIRST_ARG_VARNAME`] minus the destroy-only `unset`
    /// ([`Traits::DESTROYS_VARIABLE`]).
    ///
    /// The single membership query for the write-command consumers —
    /// loop-bound checks, dead-store cancellation, embedded-script def
    /// collection, catch-body out-vars — which previously each kept a
    /// hardcoded (and mutually inconsistent) name set.
    #[must_use]
    pub fn writes_first_arg_variable(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FIRST_ARG_VARNAME)
                && !s.traits.contains(Traits::DESTROYS_VARIABLE)
        })
    }

    /// Whether `name` **read-modify-writes** the variable named by its
    /// first argument (`append` / `lappend` / `incr` / `lset` — not the
    /// whole-value `set`): [`Traits::FIRST_ARG_VARNAME`] ∧
    /// [`Traits::READS_BEFORE_WRITE`].
    ///
    /// Drives the minifier's RMW target protection: a name-compaction
    /// must not rename a variable whose current value an RMW command is
    /// about to fold into, while a plain `set` target is rename-safe.
    #[must_use]
    pub fn rmw_first_arg_variable(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FIRST_ARG_VARNAME)
                && s.traits.contains(Traits::READS_BEFORE_WRITE)
        })
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

    /// The [`crate::traits::UNIT_LINKAGE_TRAITS`] this concrete invocation
    /// carries — the registry's answer to "does this command widen the set
    /// of callers beyond the file it appears in?".
    ///
    /// `PROVIDES_PACKAGE` (`package provide` / `ifneeded`) and
    /// `EXPORTS_COMMAND` (`namespace export`, `namespace ensemble`) say the
    /// file publishes an API surface; `LOADS_EXTERNAL_UNIT` (`source`,
    /// `load`, `package require`, `auto_load`, `auto_import`, `namespace
    /// import`) says another unit's script runs in this interpreter and can
    /// call back in.  Any of the three sinks the "every caller of this
    /// file's procs is in this file" assumption that
    /// `tcl_compiler::unit_scope`'s interprocedural call-site seed rests on
    /// (issue #977).
    ///
    /// Resolved through [`Self::resolve_call`], so the subcommand word is
    /// honoured (`package provide` is a boundary, `package names` is not)
    /// and `spec.traits | sub.traits` composes exactly as
    /// [`crate::spec::SubCommand::traits`] documents. An unknown command
    /// carries no linkage — a user proc named `source` is a user proc.
    #[must_use]
    pub fn unit_linkage(&self, name: &str, args: &[&str], dialect: DialectSet) -> Traits {
        self.invocation_traits(name, args, dialect)
            .intersection(crate::traits::UNIT_LINKAGE_TRAITS)
    }

    /// Every trait this concrete invocation carries — `spec.traits |
    /// sub.traits`, composed exactly as [`crate::spec::SubCommand::traits`]
    /// documents.
    ///
    /// The invocation-level counterpart of `self.get(name).traits`: a
    /// subcommand's traits are *additive* over its parent's, so a consumer
    /// asking a trait question about a compound command must compose them.
    /// `namespace eval` / `namespace inscope` / `interp eval` carry the
    /// eval-family bits ([`Traits::EVALUATES_CODE`],
    /// [`Traits::SCRIPT_CONCATENATES_ARGS`]) on the **subcommand**, not on
    /// the `namespace` / `interp` spec, so a parent-only trait test silently
    /// misses them.
    ///
    /// Resolved through [`Self::resolve_call`], so the subcommand word is
    /// honoured; pass [`DialectSet::empty`] to skip dialect gating (the
    /// plain [`Self::get`] lookup) when the question is "what shape is this
    /// command" rather than "is it available here". An unknown command
    /// carries no traits.
    #[must_use]
    pub fn invocation_traits(&self, name: &str, args: &[&str], dialect: DialectSet) -> Traits {
        let Some(resolved) = self.resolve_call(name, args, dialect) else {
            return Traits::empty();
        };
        resolved.spec.traits | resolved.sub.map_or_else(Traits::empty, |sub| sub.traits)
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

    /// Whether `name` is **frame-sensitive**: its meaning depends on the
    /// frame or scope it executes in, so moving a call across a proc
    /// boundary (inlining it into a caller) changes behaviour.  The union
    /// of the block-unwinding ([`Traits::TERMINATES_BLOCK`]),
    /// control-transferring ([`Traits::TRANSFERS_CONTROL`]),
    /// scope-aliasing ([`Traits::CREATES_SCOPE_ALIAS`]), and
    /// barrier-creating ([`Traits::CREATES_BARRIER`]) traits.  Like
    /// [`Self::is_byte_compiled`], checks every spec registered under the
    /// name.  Drives the inline-proc code action's safety decline and the
    /// negative half of [`Self::is_splice_safe`].
    #[must_use]
    pub fn is_frame_sensitive(&self, name: &str) -> bool {
        self.by_name.get(name).is_some_and(|specs| {
            specs
                .iter()
                .any(|s| s.traits.intersects(FRAME_SENSITIVE_TRAITS))
        })
    }

    /// Every command name that is frame-sensitive ([`Self::is_frame_sensitive`]),
    /// for consumers that scan text for any member of the set (the
    /// inline-proc code action's body check) rather than querying one
    /// resolved head.
    #[must_use]
    pub fn frame_sensitive_commands(&self) -> Vec<&str> {
        self.by_name
            .iter()
            .filter_map(|(name, specs)| {
                specs
                    .iter()
                    .any(|s| s.traits.intersects(FRAME_SENSITIVE_TRAITS))
                    .then_some(name.as_str())
            })
            .collect()
    }

    /// Whether `name` is **splice-safe**: a call to it can be lifted out of
    /// a wrapper proc and spliced into any caller's frame without changing
    /// observable behaviour.  A splice-safe command is fully lowered
    /// ([`Traits::FRAMELESS_RUNTIME`] — it never falls back to the
    /// interpreter, so it needs no frame) **and** frame-independent: not
    /// frame-sensitive ([`Self::is_frame_sensitive`]) and not operating on
    /// variables by name ([`Traits::FIRST_ARG_VARNAME`] writers,
    /// [`Traits::FRAME_HASH_BUILTIN`] hash-bucket binders, and
    /// [`Traits::DYNAMIC_EVAL_BODY`] body evaluators all observe the frame
    /// they run in).  Drives the inliner's verbatim-splice eligibility;
    /// membership is pinned by `splice_safe_membership` in
    /// `tests/registry_commands.rs`.
    #[must_use]
    pub fn is_splice_safe(&self, name: &str) -> bool {
        self.get(name).is_some_and(|s| {
            s.traits.contains(Traits::FRAMELESS_RUNTIME)
                && !s.traits.intersects(
                    FRAME_SENSITIVE_TRAITS
                        .union(Traits::FIRST_ARG_VARNAME)
                        .union(Traits::FRAME_HASH_BUILTIN)
                        .union(Traits::DYNAMIC_EVAL_BODY),
                )
        })
    }

    /// How *name* crosses stack frames, if it does — the registry's
    /// [`FrameEffectSpec`](crate::frame_effect::FrameEffectSpec).
    ///
    /// The single membership test for "is this a frame-crossing command?".
    /// A consumer reads the returned descriptor to find the level word and
    /// the affected arguments; it never names `upvar` / `uplevel` / `eval`
    /// itself.
    #[must_use]
    pub fn frame_effect(&self, name: &str) -> Option<crate::frame_effect::FrameEffectSpec> {
        self.get(name).and_then(|spec| spec.frame_effect)
    }

    /// The behavioural traits in force for one **concrete call** — the
    /// command's own set, unioned with those of the subcommand `args[0]`
    /// resolves to (exact or unique-prefix), when it has subcommands.
    ///
    /// The trait counterpart of [`Self::arg_indices_for_role`], and it takes
    /// the same `args` shape (subcommand first) for the same reason: a
    /// consumer asking "does *this* call declare a namespace / evaluate code"
    /// must not have to know whether the fact lives on the ensemble or on one
    /// of its subcommands. `namespace eval` carries
    /// [`Traits::DECLARES_NAMESPACE`] on the subcommand while `namespace`
    /// itself does not, and `namespace inscope` — identical argument layout,
    /// same analyser hook — does not carry it at all.
    #[must_use]
    pub fn call_traits(&self, name: &str, args: &[&str]) -> Traits {
        let Some(spec) = self.get(name) else {
            return Traits::empty();
        };
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        sub.map_or(spec.traits, |sub| spec.traits.union(sub.traits))
    }

    /// Resolve argument indices for a given role.
    ///
    /// For subcommand-based commands (e.g. `dict create`), pass the
    /// subcommand as the first element of `args`. This is the Rust
    /// equivalent of `arg_indices_for_role()`.
    #[must_use]
    pub fn arg_indices_for_role(&self, name: &str, args: &[&str], role: ArgRole) -> Vec<usize> {
        // `CommandPrefix` positions (with their appended arities) are owned by
        // [`Self::command_prefixes`]; delegate so highlighting, param-trait
        // inference, and the call-reference extractor all read one source.
        if role == ArgRole::CommandPrefix {
            return self
                .command_prefixes(name, args)
                .into_iter()
                .map(|(i, _)| i)
                .collect();
        }
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let n = args.len();
        let mut out: Vec<usize> = Vec::new();

        // Check subcommand (exact or unique-prefix abbreviation).
        if !spec.subcommands.is_empty()
            && !args.is_empty()
            && let Some(sub) = spec.resolve_subcommand(args[0])
        {
            // Positional roles, offset by +1 for the subcommand word.
            if let Some(resolver) = sub.arg_role_resolver {
                out.extend(
                    resolver(&args[1..])
                        .into_iter()
                        .filter(|(_, r)| *r == role)
                        .map(|(i, _)| i as usize + 1),
                );
            } else {
                out.extend(
                    sub.arg_roles
                        .iter()
                        .filter(|(_, r)| *r == role)
                        .map(|(i, _)| *i as usize + 1),
                );
            }
            // Value-taking options on the subcommand (scan past the sub word).
            push_option_value_roles(&mut out, sub.options, args, 1, role);
            out.retain(|&idx| idx < n);
            return out;
        }

        // Top-level positional roles.
        if let Some(resolver) = spec.arg_role_resolver {
            out.extend(
                resolver(args)
                    .into_iter()
                    .filter(|(_, r)| *r == role)
                    .map(|(i, _)| i as usize),
            );
        } else {
            out.extend(
                spec.arg_roles
                    .iter()
                    .filter(|(_, r)| *r == role)
                    .map(|(i, _)| *i as usize),
            );
        }
        // Value-taking options carry roles at their (dynamic) value positions.
        push_option_value_roles(&mut out, spec.options, args, 0, role);
        out.retain(|&idx| idx < n);
        out
    }

    /// How a formatter should **present** argument `index` of a call to
    /// `name` — the layout fact that refines the argument's semantic
    /// [`ArgRole`] (issue #1186).
    ///
    /// Returns the declared override, or [`ArgPresentation::BlockScript`]
    /// (the default) when the spec says nothing. `for`'s `start` and `next`
    /// scripts answer [`ArgPresentation::InlineScript`], which is how the
    /// formatting engine keeps them on the header line without a
    /// `name == "for"` branch.
    ///
    /// Resolves through [`Self::get`], so the explicitly-global spelling
    /// `::for` answers identically to the bare `for` — the false negative
    /// the formatter's literal name comparison had. A command the registry
    /// does not know (a user proc, a dynamic head) answers the default,
    /// which is what leaves such a call formatted like any other command.
    ///
    /// `args` is the post-name argument list, in the same shape
    /// [`Self::arg_indices_for_role`] takes (subcommand first), so a
    /// subcommand-dispatching call resolves against the subcommand's own
    /// table with the same `+1` offset.
    #[must_use]
    pub fn arg_presentation(
        &self,
        name: &str,
        args: &[&str],
        index: usize,
    ) -> crate::presentation::ArgPresentation {
        use crate::presentation::ArgPresentation;
        let Some(spec) = self.get(name) else {
            return ArgPresentation::default();
        };
        // A dispatching subcommand owns the positions after its own word.
        if !spec.subcommands.is_empty()
            && let Some(sub) = args.first().and_then(|word| spec.resolve_subcommand(word))
            && index > 0
            && let Ok(sub_index) = u8::try_from(index - 1)
        {
            return sub
                .arg_presentation
                .iter()
                .find(|(i, _)| *i == sub_index)
                .map_or_else(ArgPresentation::default, |(_, p)| *p);
        }
        let Ok(idx) = u8::try_from(index) else {
            return ArgPresentation::default();
        };
        spec.arg_presentation
            .iter()
            .find(|(i, _)| *i == idx)
            .map_or_else(ArgPresentation::default, |(_, p)| *p)
    }

    /// The declared roles of the argument positions a call has **not**
    /// filled — the optional trailing words a caller could still append.
    ///
    /// [`Self::arg_indices_for_role`] answers "what role does each supplied
    /// argument play"; this answers the complementary question a
    /// splice-a-trailing-word quick-fix needs: "what would the *next* words
    /// mean if I added them".  Positions are returned in ascending order,
    /// starting at `args.len()` and stopping at the spec's arity ceiling, and
    /// only while the spec actually declares a role for the position — an
    /// unlimited-arity command with no declared tail role yields nothing
    /// rather than an unbounded run of `Value` slots.
    ///
    /// A subcommand-dispatching call resolves against the subcommand's own
    /// role table (offset by the subcommand word), exactly as
    /// [`Self::arg_indices_for_role`] does, so `chan gets`-shaped commands
    /// answer for the right spec.  Dynamic
    /// [`arg_role_resolver`](CommandSpec::arg_role_resolver) tables are
    /// consulted with the supplied argument list, which is what a resolver
    /// keyed on the *written* words can answer; positions the resolver does
    /// not name are simply absent from the result.
    #[must_use]
    pub fn unfilled_trailing_roles(&self, name: &str, args: &[&str]) -> Vec<(usize, ArgRole)> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        // Resolve against the subcommand when one dispatches, mirroring
        // `arg_indices_for_role`: `sub_offset` is the number of leading words
        // (the subcommand itself) the subcommand's own indices sit after.
        let sub = (!spec.subcommands.is_empty())
            .then(|| args.first().and_then(|word| spec.resolve_subcommand(word)))
            .flatten();
        let (static_roles, dynamic_roles, sub_offset) = match sub {
            Some(sub) => (sub.arg_roles, sub.arg_role_resolver, 1usize),
            None => (spec.arg_roles, spec.arg_role_resolver, 0usize),
        };
        let declared: Vec<(usize, ArgRole)> = match dynamic_roles {
            Some(resolve) => resolve(args.get(sub_offset..).unwrap_or(&[]))
                .into_iter()
                .map(|(idx, role)| (idx as usize + sub_offset, role))
                .collect(),
            None => static_roles
                .iter()
                .map(|(idx, role)| (*idx as usize + sub_offset, *role))
                .collect(),
        };
        let ceiling = usize::from(spec.arity.max);
        (args.len()..ceiling)
            .map_while(|position| {
                declared
                    .iter()
                    .find(|(idx, _)| *idx == position)
                    .map(|(_, role)| (position, *role))
            })
            .collect()
    }

    /// [`Self::arg_indices_for_role`]`(name, args, `[`ArgRole::Body`]`)`,
    /// filtered to the indices whose [`BodyKind`] is
    /// [`Plain`](BodyKind::Plain) — a body that runs in the caller's own
    /// frame (`if`/`while`/`for`/`foreach`/`switch`/`try`/`catch`/`eval`, …).
    ///
    /// `Structural` bodies (`proc`, `oo::class create`, `oo::define`,
    /// `snit::method`, `uplevel`, `namespace eval`, …) execute in a
    /// definition or different-frame dispatch context, so are excluded —
    /// a command found inside one is *not* still running in the enclosing
    /// call's scope.  This is the generic test for "is a dispatch nested in
    /// this body argument still the same lexical/dispatch context as the
    /// caller": intra-class `my`/`next`/`nextto` `TclOO` dispatch and `$obj`
    /// method dispatch recursion both need this to decide which nested
    /// script regions still belong to the same enclosing method.
    ///
    /// Respects a subcommand's own `body_kind` override (e.g. a compound
    /// command whose subcommand's body is structural even though the
    /// top-level spec defaults to `Plain`), the same resolution order
    /// [`Self::arg_indices_for_role`] uses for the subcommand's other roles.
    #[must_use]
    pub fn plain_body_arg_indices(&self, name: &str, args: &[&str]) -> Vec<usize> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let sub_body_kind = if spec.subcommands.is_empty() {
            None
        } else {
            args.first()
                .and_then(|first| spec.resolve_subcommand(first))
                .map(|sub| sub.body_kind)
        };
        let body_kind = sub_body_kind.unwrap_or(spec.body_kind);
        if body_kind != BodyKind::Plain {
            return Vec::new();
        }
        self.arg_indices_for_role(name, args, ArgRole::Body)
    }

    /// Resolve [`ArgRole::CommandPrefix`] argument positions and their
    /// [`AppendedArity`] for a concrete call.
    ///
    /// The single source of truth for command-prefix callbacks — unions the
    /// three declaration mechanisms (static [`CommandSpec::command_prefixes`]
    /// table, [`CommandSpec::command_prefix_resolver`], and `command_prefix`
    /// option values), for the top-level command or its resolved subcommand.
    /// Mirrors [`Self::arg_indices_for_role`]'s subcommand offset / `--`
    /// handling. Consumers: highlighting, find-references / call-hierarchy /
    /// call-graph recording, and the callback-arity check.
    ///
    /// For subcommand-based commands pass the subcommand as `args[0]`.
    #[must_use]
    pub fn command_prefixes(&self, name: &str, args: &[&str]) -> Vec<(usize, AppendedArity)> {
        let Some(spec) = self.get(name) else {
            return Vec::new();
        };
        let n = args.len();
        let mut out: Vec<(usize, AppendedArity)> = Vec::new();

        if !spec.subcommands.is_empty()
            && !args.is_empty()
            && let Some(sub) = spec.resolve_subcommand(args[0])
        {
            if let Some(resolver) = sub.command_prefix_resolver {
                out.extend(
                    resolver(&args[1..])
                        .into_iter()
                        .map(|(i, a)| (i as usize + 1, a)),
                );
            } else {
                out.extend(
                    sub.command_prefixes
                        .iter()
                        .map(|(i, a)| (*i as usize + 1, *a)),
                );
            }
            push_command_prefix_options(&mut out, sub.options, args, 1);
            out.retain(|&(idx, _)| idx < n);
            return out;
        }

        if let Some(resolver) = spec.command_prefix_resolver {
            out.extend(resolver(args).into_iter().map(|(i, a)| (i as usize, a)));
        } else {
            out.extend(spec.command_prefixes.iter().map(|(i, a)| (*i as usize, *a)));
        }
        push_command_prefix_options(&mut out, spec.options, args, 0);
        out.retain(|&(idx, _)| idx < n);
        out
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
            inline_codegen_hook: spec.inline_codegen_hook,
            analyser_hook: spec.analyser_hook,
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
            // Forms carry no inline hook — the inline emitters guard
            // their own applicability (arity / shape) at the dispatch
            // site, so subcommand-level wins over command-level.
            resolved.inline_codegen_hook = sub.inline_codegen_hook.or(spec.inline_codegen_hook);
            // Forms carry no analyser hook either — the analyser
            // handlers keep their own shape guards, so the
            // subcommand-level stamp wins over the command-level one.
            resolved.analyser_hook = sub.analyser_hook.or(spec.analyser_hook);
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

        // Subcommand-scoped first. Resolve the subcommand word the same way
        // ensemble dispatch does — accepting a unique prefix abbreviation
        // (`string le` ⇒ `length`) — so an abbreviated subcommand keeps its
        // `--` terminator profile, matching `arg_indices_for_role`.
        if let Some(first) = args.first()
            && let Some(sub) = if dialect.is_empty() {
                spec.resolve_subcommand(first)
            } else {
                spec.resolve_subcommand_for_dialect(first, dialect)
            }
            && sub.options.iter().any(|o| o.name == "--")
        {
            return Some(ResolvedTerminator {
                scan_start: 1,
                subcommand: Some(sub.name),
                options: sub.options,
                reserved_trailing_words: 0,
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
                reserved_trailing_words: spec.reserved_trailing_words,
            });
        }

        None
    }

    /// Resolve how a call mutates the interpreter's command table —
    /// the [`CommandTableEffect`] stamped on the command spec, or on
    /// the subcommand `first_arg` names (which wins), for the
    /// mutators `proc` / `rename` / `interp alias`.
    ///
    /// `name` is matched against the spec's own spelling only: a
    /// `::`-qualified head resolves no effect, mirroring the retired
    /// per-consumer literal matches (`cmd_name != "rename"`), which
    /// never matched a qualified spelling — callers that canonicalise
    /// first (the command-binding lattice strips a leading `::`)
    /// keep doing so before calling.  The subcommand word must match
    /// exactly (no prefix abbreviation), as those matches also did.
    #[must_use]
    pub fn command_table_effect(
        &self,
        name: &str,
        first_arg: Option<&str>,
    ) -> Option<CommandTableEffect> {
        if name.starts_with("::") {
            return None;
        }
        let spec = self.get(name)?;
        first_arg
            .and_then(|word| spec.subcommand(word))
            .and_then(|sub| sub.command_table_effect)
            .or(spec.command_table_effect)
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
    /// Trailing words (after the command name) that are never scanned as
    /// option candidates — see [`crate::spec::CommandSpec::reserved_trailing_words`].
    /// `0` for every subcommand-scoped match (no subcommand currently
    /// needs the reservation) and for any form without one declared.
    pub reserved_trailing_words: usize,
}

/// Which `TclOO` method-context keyword a command word is — the answer
/// [`CommandRegistry::method_dispatch_keyword`] returns.
///
/// The three variants are three different axes, deliberately not merged: a
/// consumer that wants "does the next word name a method" wants
/// [`Self::SelfDispatch`] alone, and one that wants "is this a call into the
/// method chain" wants `SelfDispatch | NextChain` but never
/// [`Self::Introspection`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodDispatchKind {
    /// `my` — dispatches a method on the current object; the following word
    /// is a method name on the enclosing class
    /// ([`Traits::TCLOO_SELF_DISPATCH`]).
    SelfDispatch,
    /// `next` / `nextto` — invokes the next implementation of the
    /// *currently executing* method along the receiver's method resolution
    /// order, so no word names a method. `nextto`'s first word names the
    /// *class* to resume from, marked [`ArgRole::Name`] on the spec
    /// ([`Traits::TCLOO_NEXT_CHAIN`]).
    NextChain,
    /// `self` — introspects the current invocation and dispatches nothing;
    /// its argument is a closed subcommand set, never a method name
    /// ([`Traits::TCLOO_INTROSPECTION`]).
    Introspection,
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
    /// Effective inline (value-position / catch-body) codegen hook
    /// identifier (subcommand-level wins over command-level).
    pub inline_codegen_hook: Option<InlineCodegenHookId>,
    /// Effective analyser handler-family hook identifier
    /// (subcommand-level wins over command-level).
    pub analyser_hook: Option<AnalyserHookId>,
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

    /// Effective [`VarWriteTyping`] for this resolved call: the matched
    /// subcommand's when one matched (`binary scan` destructures where the
    /// bare `binary` does not), otherwise the top-level [`CommandSpec`]'s.
    ///
    /// The compiler's type-inference pass consults this to type the
    /// variables a command writes as a side effect, rather than assuming
    /// they receive the command's return value (issue #867).
    #[must_use]
    pub fn var_write_typing(&self) -> VarWriteTyping {
        self.sub
            .map_or(self.spec.var_write_typing, |s| s.var_write_typing)
    }

    /// The result↔element-structure fact for this call — the subcommand's
    /// when one matched, else the command's. See
    /// [`crate::types::ReturnElements`].
    #[must_use]
    pub fn return_elements(&self) -> Option<crate::types::ReturnElements> {
        self.sub
            .map_or(self.spec.return_elements, |s| s.return_elements)
    }

    /// The in-place element evolution of the written variable for this call —
    /// the subcommand's when one matched, else the command's. See
    /// [`crate::types::VarElementsEffect`].
    #[must_use]
    pub fn var_elements_effect(&self) -> Option<crate::types::VarElementsEffect> {
        self.sub
            .map_or(self.spec.var_elements_effect, |s| s.var_elements_effect)
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
            .field("profile", &self.profile.map(|p| p.name))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    // -- unfilled_trailing_roles (issue #1190) ----------------------------
    //
    // The complement of `arg_indices_for_role`: what the *next* words would
    // mean, which is the question a splice-a-trailing-argument quick fix asks.

    #[test]
    fn unfilled_trailing_roles_reports_the_optional_capture_variables() {
        let reg = CommandRegistry::build_default();
        // `catch {body}` leaves both `VarWrite` slots open.
        assert_eq!(
            reg.unfilled_trailing_roles("catch", &["{body}"]),
            vec![(1, ArgRole::VarWrite), (2, ArgRole::VarWrite)]
        );
        // One supplied leaves one.
        assert_eq!(
            reg.unfilled_trailing_roles("catch", &["{body}", "res"]),
            vec![(2, ArgRole::VarWrite)]
        );
        // Fully supplied leaves none.
        assert!(
            reg.unfilled_trailing_roles("catch", &["{body}", "res", "opts"])
                .is_empty()
        );
    }

    #[test]
    fn unfilled_trailing_roles_stops_at_the_arity_ceiling() {
        let reg = CommandRegistry::build_default();
        // Every position past `catch`'s maximum of three is absent, however
        // many are asked for.
        let roles = reg.unfilled_trailing_roles("catch", &["{body}"]);
        assert!(roles.iter().all(|(index, _)| *index < 3), "{roles:?}");
    }

    #[test]
    fn unfilled_trailing_roles_is_empty_for_an_unknown_command() {
        let reg = CommandRegistry::build_default();
        assert!(
            reg.unfilled_trailing_roles("no_such_command", &[])
                .is_empty()
        );
    }

    #[test]
    fn unfilled_trailing_roles_stops_at_an_undeclared_position() {
        let reg = CommandRegistry::build_default();
        // `puts` declares no trailing role a caller could fill with a
        // *known* meaning, so nothing is offered rather than an unbounded
        // run of `Value` slots.
        assert!(reg.unfilled_trailing_roles("puts", &["hello"]).is_empty());
    }

    /// `SAFE_INTERP_HIDDEN` and `TRANSFERS_CONTROL` were the same bit.
    ///
    /// Both were spelled `1 << 61`, so the 65th trait silently aliased the
    /// 61st. Because `FRAME_SENSITIVE_TRAITS` unions in `TRANSFERS_CONTROL`,
    /// every safe-interp-hidden command — `file`, `source`, `encoding`,
    /// `open`, … — read as frame-sensitive and had the inline-proc code
    /// action suppressed, while `break`/`continue`/`yield`/`yieldto`/
    /// `tailcall` read as safe-interp-hidden.
    ///
    /// The two are now separate enum variants, so the aliasing is
    /// unrepresentable rather than merely fixed. This pins the *behaviour*
    /// that was wrong, which a type-level guarantee alone does not cover.
    #[test]
    fn safe_interp_hidden_is_not_control_transfer() {
        let reg = CommandRegistry::build_default();
        assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);

        for name in ["break", "continue", "yield", "yieldto", "tailcall"] {
            let spec = reg.get(name).expect("control-transfer command");
            assert!(
                spec.traits.contains(Traits::TRANSFERS_CONTROL),
                "{name} must transfer control"
            );
            assert!(
                !spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
                "{name} is not hidden in a safe interpreter"
            );
            assert!(reg.is_frame_sensitive(name), "{name} is frame-sensitive");
        }

        // Safe-hidden commands carrying no *other* frame-sensitive trait.
        // These are the ones the alias was wrongly marking: each had the
        // inline-proc code action suppressed on it.
        for name in [
            "file", "encoding", "open", "socket", "exec", "cd", "pwd", "glob", "load", "unload",
        ] {
            let spec = reg.get(name).expect("safe-hidden command");
            assert!(
                spec.traits.contains(Traits::SAFE_INTERP_HIDDEN),
                "{name} is hidden in a safe interpreter"
            );
            assert!(
                !spec.traits.contains(Traits::TRANSFERS_CONTROL),
                "{name} does not transfer control"
            );
            assert!(
                !reg.is_frame_sensitive(name),
                "{name} must not be frame-sensitive — the alias suppressed the \
                 inline-proc code action on it"
            );
        }

        // The correction must not swing too far: `source` and `exit` are
        // frame-sensitive on their own merits (a barrier and a block
        // terminator respectively), and stay so.
        for name in ["source", "exit"] {
            assert!(
                reg.is_frame_sensitive(name),
                "{name} is frame-sensitive independently of the trait alias"
            );
        }
    }

    use super::*;

    #[test]
    fn get_for_dialect_picks_the_most_specific_visible_spec() {
        // §5.3/D6 — the single selection rule. Three specs of one name,
        // registered deliberately most-specific-FIRST so the old
        // last-match rule would pick the catch-all:
        //   1. a TCL86-scoped spec (tightest),
        //   2. a wider TCL86|TCL90 spec,
        //   3. a catch-all (`dialects: None`).
        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: "d6_probe",
            dialects: Some(DialectSet::TCL86),
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_probe",
            dialects: Some(DialectSet::TCL86.union(DialectSet::TCL90)),
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_probe",
            dialects: None,
            ..CommandSpec::DEFAULT
        });

        // Scoped beats catch-all, tighter beats wider — even though the
        // catch-all was registered last.
        let under_86 = reg.get_for_dialect("d6_probe", DialectSet::TCL86);
        assert_eq!(under_86.and_then(|s| s.dialects), Some(DialectSet::TCL86));
        // Under 9.0 the tightest visible spec is the two-bit one.
        let under_90 = reg.get_for_dialect("d6_probe", DialectSet::TCL90);
        assert_eq!(
            under_90.and_then(|s| s.dialects),
            Some(DialectSet::TCL86.union(DialectSet::TCL90))
        );
        // Where no scoped spec is visible, the catch-all still resolves.
        let under_84 = reg.get_for_dialect("d6_probe", DialectSet::TCL84);
        assert!(under_84.is_some_and(|s| s.dialects.is_none()));
    }

    #[test]
    fn get_for_dialect_breaks_specificity_ties_by_last_registration() {
        // Curated pack overrides re-register a name at the same scope; the
        // later registration must keep winning (the old `.rev()` guarantee,
        // preserved as the D6 tie-break).
        let mut reg = CommandRegistry::build_default();
        reg.insert(CommandSpec {
            name: "d6_tie",
            dialects: Some(DialectSet::TCL86),
            arity: Arity::exact(1), // base data
            ..CommandSpec::DEFAULT
        });
        reg.insert(CommandSpec {
            name: "d6_tie",
            dialects: Some(DialectSet::TCL86),
            arity: Arity::exact(2), // curated override
            ..CommandSpec::DEFAULT
        });
        let won = reg
            .get_for_dialect("d6_tie", DialectSet::TCL86)
            .expect("d6_tie resolves");
        assert_eq!(won.arity, Arity::exact(2), "later registration wins ties");
    }

    #[test]
    fn instance_method_walks_superclasses_breadth_first() {
        use crate::spec::{ObjectClassSpec, SubCommand};

        // Diamond hierarchy: Diamond → {Left, Right} → Base. Both Left and
        // Right (and Base) define `m`; a breadth-first walk in declaration
        // order must find Left's `m` first, whereas a `Vec::pop` LIFO walk
        // would reverse the siblings and return Right's.
        static M_LEFT: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "left",
            ..SubCommand::DEFAULT
        }];
        static M_RIGHT: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "right",
            ..SubCommand::DEFAULT
        }];
        static M_BASE: [SubCommand; 1] = [SubCommand {
            name: "m",
            detail: "base",
            ..SubCommand::DEFAULT
        }];
        static DIAMOND: ObjectClassSpec = ObjectClassSpec {
            class_name: "Diamond",
            instance_methods: &[],
            superclasses: &["Left", "Right"],
            allow_unknown_methods: false,
        };
        static LEFT: ObjectClassSpec = ObjectClassSpec {
            class_name: "Left",
            instance_methods: &M_LEFT,
            superclasses: &["Base"],
            allow_unknown_methods: false,
        };
        static RIGHT: ObjectClassSpec = ObjectClassSpec {
            class_name: "Right",
            instance_methods: &M_RIGHT,
            superclasses: &["Base"],
            allow_unknown_methods: false,
        };
        static BASE: ObjectClassSpec = ObjectClassSpec {
            class_name: "Base",
            instance_methods: &M_BASE,
            superclasses: &[],
            allow_unknown_methods: false,
        };

        let mut reg = CommandRegistry::build_default();
        for oc in [&DIAMOND, &LEFT, &RIGHT, &BASE] {
            reg.insert(CommandSpec {
                name: oc.class_name,
                object_class: Some(oc),
                ..CommandSpec::DEFAULT
            });
        }

        let resolved = reg
            .instance_method("Diamond", "m")
            .expect("method resolves");
        assert_eq!(
            resolved.detail, "left",
            "breadth-first, declaration-ordered walk visits Left before Right"
        );
    }

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
        // tcl9.0 (TIP 114) reads leading zeros as decimal; everything else
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
        // RUST_ISSUE_024: tcl9.1 keeps the TIP 114 decimal rule; a tcl9.1-only
        // registry (loads TCL91, not TCL90) must still read leading zeros as
        // decimal.
        let mut reg91 = CommandRegistry::build_default();
        reg91.load_dialect(DialectSet::TCL91);
        assert!(!reg91.leading_zero_is_octal(), "tcl9.1 should be decimal");
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
        // `const` (Tcl 9.0, TIP 677) joins the list: it used to carry
        // `dialects: None` as a workaround to reach iRules events, which
        // wrongly made it appear valid in 8.4/8.5/8.6 too. The registry-wide
        // explicit-dialect sweep corrected it to `TCL90_PLUS` — it does not
        // exist in iRules' embedded Tcl 8.4.6, so it is (correctly) neither
        // pre-9.0 nor iRules-visible.
        for name in ["foreachLine", "readFile", "writeFile", "lpop", "const"] {
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
            assert!(!spec.supports_dialect(DialectSet::IRULES));
        }
    }

    #[test]
    fn every_command_has_hover_with_manpage_source() {
        // Every registered command must carry a hover snippet with a non-empty
        // summary and a manpage/source attribution. A short allowlist covers
        // internal pseudo-commands and dialect placeholders that have no user
        // documentation.
        // The four regex-quote spellings are internal idiom-recognition
        // entries for the taint analyser (T103), not real Tcl commands --
        // no manpage exists to cite (re_quote.html/.htm 404s on every
        // tcl-lang.org tree for 8.4-9.1 alike; see re_quote.rs's own doc
        // comment for the full explanation).
        const HOVERLESS_OK: &[&str] = &[
            "disabled_in_irules",
            "re_quote",
            "regex_quote",
            "regex::quote",
            "regexp::quote",
        ];
        let reg = CommandRegistry::build_default();
        let mut missing_hover = Vec::new();
        let mut missing_source = Vec::new();
        for name in reg.command_names() {
            if HOVERLESS_OK.contains(&name) {
                continue;
            }
            let spec = reg.get(name).expect("registered");
            match &spec.hover {
                None => missing_hover.push(name.to_string()),
                Some(h) => {
                    if h.summary.trim().is_empty() || h.source.trim().is_empty() {
                        missing_source.push(name.to_string());
                    }
                }
            }
        }
        missing_hover.sort();
        missing_source.sort();
        assert!(
            missing_hover.is_empty(),
            "commands without a hover snippet: {missing_hover:?}",
        );
        assert!(
            missing_source.is_empty(),
            "commands with an empty hover summary or manpage source: {missing_source:?}",
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
    fn uplevel_body_arg_role_skips_optional_level() {
        // Issue #837: `uplevel ?level? {body}` — the body word's index depends
        // on whether a leading `level` word is present. The registry resolver
        // is the single source of truth every body consumer (semantic tokens,
        // green-tree descent, SSA) queries.
        let reg = CommandRegistry::build_default();
        // Literal relative level → body at 1.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["1", "{set x 1}"], ArgRole::Body),
            vec![1]
        );
        // Absolute `#0` level → body at 1.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["#0", "{set x 1}"], ArgRole::Body),
            vec![1]
        );
        // No level → body at 0.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["{set x 1}"], ArgRole::Body),
            vec![0]
        );
        // Dynamic level followed by a script → body at 1.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["$lvl", "{set x 1}"], ArgRole::Body),
            vec![1]
        );
        // A lone dynamic word is the body itself (implicit level 1) → body at 0.
        assert_eq!(
            reg.arg_indices_for_role("uplevel", &["$body"], ArgRole::Body),
            vec![0]
        );
        // Bodyless `uplevel 1` (a wrong-#args error) exposes no body word — the
        // literal level must not be mis-tagged as a script.
        assert!(
            reg.arg_indices_for_role("uplevel", &["1"], ArgRole::Body)
                .is_empty()
        );
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

    /// Issue #1186 — `for`'s three script arguments share the semantic
    /// [`ArgRole::Body`], and the *presentation* fact is what separates the
    /// trailing body (block-expanded) from `start` / `next` (inline).
    #[test]
    fn for_declares_inline_presentation_for_start_and_next() {
        use crate::presentation::ArgPresentation;
        let reg = CommandRegistry::build_default();
        let args = ["{set i 0}", "{$i < 3}", "{incr i}", "{puts $i}"];
        // Semantics unchanged: all three scripts are still bodies.
        assert_eq!(
            reg.arg_indices_for_role("for", &args, ArgRole::Body),
            vec![0, 2, 3]
        );
        // Presentation splits them.
        assert_eq!(
            reg.arg_presentation("for", &args, 0),
            ArgPresentation::InlineScript
        );
        assert_eq!(
            reg.arg_presentation("for", &args, 2),
            ArgPresentation::InlineScript
        );
        assert_eq!(
            reg.arg_presentation("for", &args, 3),
            ArgPresentation::BlockScript
        );
        assert!(reg.arg_presentation("for", &args, 3).is_block());
        // The absolute global spelling resolves to the same spec.
        assert_eq!(
            reg.arg_presentation("::for", &args, 0),
            ArgPresentation::InlineScript
        );
        // A command with nothing to say answers the block default, and so
        // does one the registry does not know at all.
        assert_eq!(
            reg.arg_presentation("while", &["{$x}", "{body}"], 1),
            ArgPresentation::BlockScript
        );
        assert_eq!(
            reg.arg_presentation("no::such::command", &["{a}"], 0),
            ArgPresentation::BlockScript
        );
    }

    // -- Option-value roles (Phase 1) ------------------------------------

    fn opt(name: &'static str, value: crate::hover::OptionValue) -> crate::hover::OptionSpec {
        crate::hover::OptionSpec {
            name,
            value,
            ..crate::hover::OptionSpec::DEFAULT
        }
    }

    fn opt_with_alias(
        name: &'static str,
        aliases: &'static [&'static str],
        value: crate::hover::OptionValue,
    ) -> crate::hover::OptionSpec {
        crate::hover::OptionSpec {
            name,
            value,
            aliases,
            ..crate::hover::OptionSpec::DEFAULT
        }
    }

    fn indices(options: &[crate::hover::OptionSpec], args: &[&str], role: ArgRole) -> Vec<usize> {
        let mut out = Vec::new();
        push_option_value_roles(&mut out, options, args, 0, role);
        out
    }

    #[test]
    fn option_value_role_emits_body_index() {
        use crate::hover::OptionValue;
        let options = [
            opt("-command", OptionValue::script()),
            opt("-flag", OptionValue::flag()),
        ];
        let args = ["-command", "{puts hi}", "-flag", "x"];
        // The `-command` value is a Body; the flag consumes nothing.
        assert_eq!(indices(&options, &args, ArgRole::Body), vec![1]);
        // It is not a generic Value role.
        assert!(indices(&options, &args, ArgRole::Value).is_empty());
    }

    #[test]
    fn option_value_fixed_arity_emits_n_indices() {
        use crate::hover::OptionValue;
        let options = [opt("-rect", OptionValue::fixed(4, ArgRole::Value, "coord"))];
        let args = ["-rect", "1", "2", "3", "4", "tail"];
        assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3, 4]);
    }

    /// `OptionArity::Hook` equivalent of the old bare `Rest` variant —
    /// consumes every remaining word, always valid.
    fn rest_value(args: &[&str], start: usize) -> crate::hover::OptionValueOutcome {
        crate::hover::OptionValueOutcome {
            words: args.len() - start,
            invalid: None,
        }
    }

    #[test]
    fn option_value_rest_arity_stops_at_terminator() {
        let rest = crate::hover::OptionSpec {
            name: "-rest",
            value: crate::hover::OptionValue::Takes(crate::hover::OptionArg {
                arity: crate::hover::OptionArity::Hook(rest_value),
                ..crate::hover::OptionArg::DEFAULT
            }),
            ..crate::hover::OptionSpec::DEFAULT
        };
        let options = [rest];
        let args = ["-rest", "a", "b", "c", "--", "d"];
        // Consumes a, b, c up to `--`, not d.
        assert_eq!(indices(&options, &args, ArgRole::Value), vec![1, 2, 3]);
    }

    #[test]
    fn option_value_terminator_stops_scan() {
        use crate::hover::OptionValue;
        let options = [opt("-command", OptionValue::script())];
        let args = ["--", "-command", "{x}"];
        assert!(indices(&options, &args, ArgRole::Body).is_empty());
    }

    #[test]
    fn option_value_two_way_var_name_emits_for_both_roles() {
        use crate::hover::OptionValue;
        let options = [opt("-textvariable", OptionValue::var_name())];
        let args = ["-textvariable", "myvar"];
        assert_eq!(indices(&options, &args, ArgRole::VarWrite), vec![1]);
        assert_eq!(indices(&options, &args, ArgRole::VarRead), vec![1]);
    }

    #[test]
    fn option_value_alias_matches_and_dynamic_flag_skipped() {
        use crate::hover::OptionValue;
        let options = [opt_with_alias("-command", &["-cmd"], OptionValue::script())];
        // Alias resolves to the same value role.
        assert_eq!(indices(&options, &["-cmd", "{x}"], ArgRole::Body), vec![1]);
        // A `$var` in flag position isn't an option name → treated as a
        // positional, so the real `-command` after it still resolves.
        assert_eq!(
            indices(&options, &["$opt", "v", "-command", "{x}"], ArgRole::Body),
            vec![3]
        );
    }

    #[test]
    fn options_do_not_leak_into_positional_role_queries() {
        // A real command with value-taking options but no role annotations
        // must not surface any option value under Body/VarWrite (inert until
        // annotated).
        let reg = CommandRegistry::build_default();
        let b = reg.arg_indices_for_role("entry", &[".e", "-width", "10"], ArgRole::Body);
        assert!(b.is_empty(), "{b:?}");
    }

    #[test]
    fn command_prefix_option_is_captured_but_never_a_body() {
        // `lsort -command cmp {a b}` — the `-command` value is a CommandPrefix
        // (Phase 6), so it is captured under that role but never returned for a
        // Body query, i.e. a bareword prefix is not recursed as a script.
        let reg = CommandRegistry::build_default();
        let args = ["-command", "cmp", "{a b}"];
        assert!(
            reg.arg_indices_for_role("lsort", &args, ArgRole::Body)
                .is_empty(),
            "a command prefix must not be recursed as a body",
        );
        assert_eq!(
            reg.arg_indices_for_role("lsort", &args, ArgRole::CommandPrefix),
            vec![1],
            "the -command value should carry the CommandPrefix role",
        );
    }

    #[test]
    fn command_prefixes_carry_verified_arities() {
        // Ground-truthed vs real tclsh (stable 8.6→9.0). `command_prefixes`
        // is the single source of truth for both the position and the
        // appended arity.
        let reg = CommandRegistry::build_default();
        // `lsort -command cmp {a b}` → cmp invoked as `cmp x y` (2 appended).
        assert_eq!(
            reg.command_prefixes("lsort", &["-command", "cmp", "{a b}"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // `socket -server accept 9000` → accept invoked as `accept ch a p` (3).
        assert_eq!(
            reg.command_prefixes("socket", &["-server", "accept", "9000"]),
            vec![(1, AppendedArity::Exactly(3))],
        );
        // Positional (migrated from arg_roles): `tcltest::customMatch mode cmd`
        // → `cmd expected actual` (2).
        assert_eq!(
            reg.command_prefixes("tcltest::customMatch", &["exact", "cmp"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // Dynamic resolver (migrated): `selection handle window cmd` → the
        // last arg, invoked as `cmd offset maxChars` (2).
        assert_eq!(
            reg.command_prefixes("selection", &["handle", ".w", "getData"]),
            vec![(2, AppendedArity::Exactly(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_core_callback_commands() {
        // Ground-truthed vs real tclsh 8.6/9.0 (stable). Locks in the Phase-2
        // coverage of trace / interp / tcllib callbacks.
        let reg = CommandRegistry::build_default();
        // `trace add variable v w cb` → cb(name1 name2 op) = 3.
        assert_eq!(
            reg.command_prefixes("trace", &["add", "variable", "v", "write", "cb"]),
            vec![(4, AppendedArity::Exactly(3))],
        );
        // `trace add command c ops cb` → cb(old new op) = 3.
        assert_eq!(
            reg.command_prefixes("trace", &["add", "command", "c", "rename", "cb"]),
            vec![(4, AppendedArity::Exactly(3))],
        );
        // `trace add execution c ops cb` → 2..4 args ⇒ AtLeast(2).
        assert_eq!(
            reg.command_prefixes("trace", &["add", "execution", "c", "enter", "cb"]),
            vec![(4, AppendedArity::AtLeast(2))],
        );
        // Deprecated `trace variable v ops cb` → cb at index 3, 3 appended args.
        assert_eq!(
            reg.command_prefixes("trace", &["variable", "v", "write", "cb"]),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `interp alias {} a {} target x` (create form) → target at index 3,
        // variadic. The 2-arg query form has no target.
        assert_eq!(
            reg.command_prefixes("interp", &["alias", "{}", "a", "{}", "target"]),
            vec![(4, AppendedArity::Unknown)],
        );
        assert!(
            reg.command_prefixes("interp", &["alias", "{}", "a"])
                .is_empty(),
            "the interp-alias query form has no command prefix",
        );
        // tcllib: `struct::list filter seq cb` (1), `map seq cb` (1),
        // `fold seq init cb` (2).
        assert_eq!(
            reg.command_prefixes("struct::list", &["filter", "$s", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("struct::list", &["fold", "$s", "0", "cb"]),
            vec![(3, AppendedArity::Exactly(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_deferred_core_commands() {
        // Version-gated / resolver-driven callback tails wired in the deferred
        // pass. Ground-truthed vs tclsh 9.0 (all are 8.7/9.0-era surfaces).
        let reg = CommandRegistry::build_default();

        // `regsub -command re str cmdPrefix ?var?` (TIP 463, 9.0): with
        // `-command` the subSpec slot is a prefix called per match with the
        // whole match + capture groups appended (variadic ⇒ AtLeast(1)). The
        // slot index tracks the leading-switch shift.
        assert_eq!(
            reg.command_prefixes("regsub", &["-command", "re", "s", "cb"]),
            vec![(3, AppendedArity::AtLeast(1))],
        );
        assert_eq!(
            reg.command_prefixes("regsub", &["-all", "-command", "re", "s", "cb"]),
            vec![(4, AppendedArity::AtLeast(1))],
        );
        // `-c` is NOT an abbreviation of `-command`: regsub's switch table
        // resolves with Tcl_GetIndexFromObj(..., TCL_EXACT, ...), confirmed
        // live (tclsh 8.6.14: `regsub -c {a} aaa X y` -> `bad option "-c"`)
        // and against the real `Tcl_RegsubObjCmd` C source. Only the exact
        // spelling `-command` enables command-prefix mode.
        assert_eq!(
            reg.command_prefixes("regsub", &["-c", "re", "s", "cb"]),
            Vec::new(),
        );
        // Without `-command`, subSpec is a replacement template, not a prefix.
        assert!(
            reg.command_prefixes("regsub", &["re", "s", "template"])
                .is_empty(),
            "plain regsub subSpec is not a command prefix",
        );
        // `--` terminates switches, so a following `-command`-looking word is a
        // pattern, not the flag.
        assert!(
            reg.command_prefixes("regsub", &["--", "-command", "s", "template"])
                .is_empty(),
            "`--` disables -command detection",
        );

        // `namespace unknown handler` → handler(cmd ?arg...?) = AtLeast(1). The
        // zero-arg query form carries no prefix.
        assert_eq!(
            reg.command_prefixes("namespace", &["unknown", "handler"]),
            vec![(1, AppendedArity::AtLeast(1))],
        );
        assert!(
            reg.command_prefixes("namespace", &["unknown"]).is_empty(),
            "the namespace-unknown query form has no command prefix",
        );

        // `package unknown handler` → handler(name requirement ?requirement
        // ...?) = AtLeast(2): verified empirically on tclsh 8.6.14 that Tcl
        // always appends the package name *plus* at least one requirement
        // word, synthesizing a "0-" placeholder when `package require` was
        // given none itself — never just the bare name, so AtLeast(2), not
        // AtLeast(1) (see the fuller note on `package_.rs`'s `unknown`
        // subcommand).
        assert_eq!(
            reg.command_prefixes("package", &["unknown", "handler"]),
            vec![(1, AppendedArity::AtLeast(2))],
        );
        assert!(
            reg.command_prefixes("package", &["unknown"]).is_empty(),
            "the package-unknown query form has no command prefix",
        );

        // `coroinject coroName command ?arg...?` — per the Tcl 9.0/9.1
        // coroutine(n) manpage, exactly two more words are appended when the
        // injected command runs: the name of the command that suspended the
        // coroutine (yield or yieldto) and its current resumption value.
        assert_eq!(
            reg.command_prefixes("coroinject", &["myCoro", "cb", "x"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        // `coroprobe coroName command ?arg...?` runs command immediately
        // inside the suspended coroutine (not deferred through a yield/
        // yieldto resumption), so no fixed extra-argument count is
        // documented -- it stays a reference-only prefix (Unknown).
        assert_eq!(
            reg.command_prefixes("coroprobe", &["myCoro", "cb"]),
            vec![(1, AppendedArity::Unknown)],
        );

        // `chan create mode cmdPrefix` / `chan push channelId cmdPrefix` — the
        // reflected-channel/transform handler is invoked as `cmdPrefix
        // subcommand handle ?args...?` ⇒ AtLeast(2).
        assert_eq!(
            reg.command_prefixes("chan", &["create", "rw", "handler"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );
        assert_eq!(
            reg.command_prefixes("chan", &["push", "$ch", "xform"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );
    }

    #[test]
    fn command_prefixes_cover_tcllib_callbacks() {
        // tcllib callback tails wired in the deferred pass.  Fixed arities are
        // ground-truthed against the package man pages; ambiguous/variadic tails
        // are Unknown (reference-only).
        let reg = CommandRegistry::build_default();

        // `struct::list split seq cmdprefix` — the twin of filter: cmdprefix(el).
        assert_eq!(
            reg.command_prefixes("struct::list", &["split", "$s", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        // `fileutil::find basedir filtercmd` — filtercmd(name).  Shorter forms
        // carry no prefix (idx≥argc dropped).
        assert_eq!(
            reg.command_prefixes("fileutil::find", &["/base", "flt"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert!(
            reg.command_prefixes("fileutil::find", &["/base"])
                .is_empty(),
            "the basedir-only fileutil::find form has no filter prefix",
        );

        // generator functional ops: reference-only (multi-value yield ⇒ Unknown).
        assert_eq!(
            reg.command_prefixes("generator", &["map", "fn", "$g"]),
            vec![(1, AppendedArity::Unknown)],
        );
        assert_eq!(
            reg.command_prefixes("generator", &["filter", "pred", "$g"]),
            vec![(1, AppendedArity::Unknown)],
        );

        // math::calculus func callbacks (man-page-pinned fixed arities).
        assert_eq!(
            reg.command_prefixes("math::calculus::integral", &["0", "1", "100", "f"]),
            vec![(3, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("math::calculus::integral3D", &["xi", "yi", "zi", "f"]),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `newtonRaphson func deriv initval` — two prefixes, each f(x).
        assert_eq!(
            reg.command_prefixes("math::calculus::newtonRaphson", &["f", "d", "0.5"]),
            vec![
                (0, AppendedArity::Exactly(1)),
                (1, AppendedArity::Exactly(1)),
            ],
        );
        // `math::probopt::pso function bounds ?args?` — objective(coordVec).
        assert_eq!(
            reg.command_prefixes("math::probopt::pso", &["obj", "bnds", "-iter", "50"]),
            vec![(0, AppendedArity::Exactly(1))],
        );

        // log message-writer callbacks — `cmd level text` (Exactly(2)).
        assert_eq!(
            reg.command_prefixes("log::lvCmd", &["debug", "writer"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("log::lvCmdForall", &["writer"]),
            vec![(0, AppendedArity::Exactly(2))],
        );
        // `uevent::bind tag event command` — command(tag event ?details?).
        assert_eq!(
            reg.command_prefixes("uevent::bind", &["tag", "ev", "cb"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );

        // `hook bind subject hook observer binding` — the 4-word set form names
        // a command prefix (Unknown appended: the count is whatever the matching
        // `hook call` passes); the shorter query forms name none.
        assert_eq!(
            reg.command_prefixes("hook", &["bind", "sub", "hk", "obs", "cb"]),
            vec![(4, AppendedArity::Unknown)],
        );
        assert!(
            reg.command_prefixes("hook", &["bind", "sub", "hk", "obs"])
                .is_empty(),
            "the 3-arg `hook bind` query form has no callback prefix",
        );

        // `processman::onexit id cmd` — `cmd` is a deferred *script* (`eval $cmd`,
        // 0 appended), NOT a command prefix: it must declare none.
        assert!(
            reg.command_prefixes("processman::onexit", &["$pid", "cb"])
                .is_empty(),
            "processman::onexit cmd is a script body, not a command prefix",
        );
        assert_eq!(
            reg.get("processman::onexit")
                .and_then(|s| s.arg_roles.iter().find(|(i, _)| *i == 1))
                .map(|(_, r)| *r),
            Some(ArgRole::Body),
            "processman::onexit cmd (index 1) must carry the Body script role",
        );
    }

    #[test]
    fn command_prefixes_cover_option_value_callbacks() {
        // Option-value callbacks — the prefix is the value of a named `-flag`,
        // resolved through each command's `OptionSpec` array.  Arities are
        // ground-truthed against tcllib source (verified by re-running the exact
        // `uplevel`/`eval`/`{*}` idioms in tclsh — `uplevel`/`eval` re-split a
        // `[list …]` word, `[list {*}pfx {*}args]` keeps one word per element).
        let reg = CommandRegistry::build_default();

        // `mime::getbody token -command cb` — async body callback.  uplevel
        // re-splits `[list end]` → 1 word, `[list data $c]` → 2 ⇒ AtLeast(1).
        assert_eq!(
            reg.command_prefixes("mime::getbody", &["tok", "-command", "cb"]),
            vec![(2, AppendedArity::AtLeast(1))],
        );
        // `-decode` before `-command` is a value-less flag: the scan skips it
        // without swallowing the callback.
        assert_eq!(
            reg.command_prefixes("mime::getbody", &["tok", "-decode", "-command", "cb"]),
            vec![(3, AppendedArity::AtLeast(1))],
        );

        // `smtp::sendmessage tok -tlspolicy pol` — `eval $pol [list $code]
        // [list $diag]` ⇒ Exactly(2).  A preceding value-taking option's value
        // is skipped, not mistaken for the callback.
        assert_eq!(
            reg.command_prefixes("smtp::sendmessage", &["tok", "-tlspolicy", "pol"]),
            vec![(2, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes(
                "smtp::sendmessage",
                &["tok", "-servers", "mail.x", "-tlspolicy", "pol"],
            ),
            vec![(4, AppendedArity::Exactly(2))],
        );

        // `comm::comm send -command cb id cmd` — 7 `-key value` reply pairs ⇒
        // Exactly(14).  Subcommand-relative option scan offsets by 1.
        assert_eq!(
            reg.command_prefixes("comm::comm", &["send", "-command", "cb", "id", "cmd"]),
            vec![(2, AppendedArity::Exactly(14))],
        );

        // `bibtex::parse` — every callback prepends the parser token, then its
        // own payload words: `-command`/`-*command` ⇒ Exactly(2), except
        // `-recordcommand` ⇒ Exactly(4) (token type key recdata).
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-recordcommand", "cb", "text"]),
            vec![(1, AppendedArity::Exactly(4))],
        );
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-command", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("bibtex::parse", &["-progresscommand", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );

        // `tcl::chan::halfpipe` — clean `[list {*}pfx {*}args]` idiom: write
        // appends (chan bytes) ⇒ 2, empty/close append (chan) ⇒ 1.  No
        // `-read-command` exists.
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-write-command", "cb"]),
            vec![(1, AppendedArity::Exactly(2))],
        );
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-close-command", "cb"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-empty-command", "cb"]),
            vec![(1, AppendedArity::Exactly(1))],
        );
        assert!(
            reg.command_prefixes("tcl::chan::halfpipe", &["-read-command", "cb"])
                .is_empty(),
            "halfpipe has no -read-command",
        );
    }

    #[test]
    fn instance_method_command_prefixes_cover_struct_graph_and_tree() {
        // Object-instance method callbacks — the prefix is on a method of a
        // created object command (`$g walk … -command cb`, `$t walkproc … cb`),
        // resolved through the class's ObjectClassSpec.  Indices are relative to
        // the words after the method name.
        let reg = CommandRegistry::build_default();

        // struct::graph `walk node … -command cb` — option-value prefix,
        // Exactly(3) (action graphName node).
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::graph",
                "walk",
                &["root", "-order", "pre", "-command", "cb"],
            ),
            vec![(4, AppendedArity::Exactly(3))],
        );
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::graph",
                "walk",
                &["root", "-command", "cb"]
            ),
            vec![(2, AppendedArity::Exactly(3))],
        );

        // struct::tree `walkproc node … cmdprefix` — trailing positional prefix
        // (resolver), Exactly(3) (tree node action).  The prefix is the final
        // word regardless of intervening `-order`/`-type` options.
        assert_eq!(
            reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root", "cb"]),
            vec![(1, AppendedArity::Exactly(3))],
        );
        assert_eq!(
            reg.instance_method_command_prefixes(
                "struct::tree",
                "walkproc",
                &["root", "-type", "dfs", "cb"],
            ),
            vec![(3, AppendedArity::Exactly(3))],
        );
        // `walkproc node` with no prefix word yet names none.
        assert!(
            reg.instance_method_command_prefixes("struct::tree", "walkproc", &["root"])
                .is_empty(),
            "a walkproc with only the node names no prefix",
        );
        // An unmodelled method / class resolves to nothing.
        assert!(
            reg.instance_method_command_prefixes("struct::graph", "get", &["x"])
                .is_empty(),
            "an unmodelled instance method has no command prefix",
        );
    }

    #[test]
    fn tk_command_options_classified_prefix_vs_script() {
        // The Tk `script()→command_prefix` conversion (separate commit, highest
        // risk).  Locks in the classification: appended-arg callbacks are
        // prefixes (references / call-graph / W123 / arity); verbatim scripts and
        // percent-substitution callbacks stay `script()` (Body, not recorded as a
        // reference).  Ground truth: Tk 8.6/9.0 man pages.
        let reg = CommandRegistry::build_default();

        // PREFIXES — scroll callbacks append `first last` (2).
        for widget in ["listbox", "text", "canvas", "entry", "spinbox"] {
            assert_eq!(
                reg.command_prefixes(widget, &[".w", "-xscrollcommand", "cb"]),
                vec![(2, AppendedArity::Exactly(2))],
                "{widget} -xscrollcommand must be a prefix appending 2",
            );
        }
        // scale / ttk::scale append the new value (1).
        assert_eq!(
            reg.command_prefixes("scale", &[".s", "-command", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        assert_eq!(
            reg.command_prefixes("ttk::scale", &[".s", "-command", "cb"]),
            vec![(2, AppendedArity::Exactly(1))],
        );
        // scrollbar -command: `moveto frac` (2) or `scroll n units` (3) ⇒ AtLeast(2).
        assert_eq!(
            reg.command_prefixes("scrollbar", &[".sb", "-command", "cb"]),
            vec![(2, AppendedArity::AtLeast(2))],
        );
        // menu -tearoffcommand appends the parent + torn-off menu paths (2).
        assert_eq!(
            reg.command_prefixes("menu", &[".m", "-tearoffcommand", "cb"]),
            vec![(2, AppendedArity::Exactly(2))],
        );

        // NOT prefixes — verbatim action scripts and percent-substitution
        // callbacks are `script()`, never recorded as a command reference.
        for (widget, opt) in [
            ("button", "-command"),
            ("checkbutton", "-command"),
            ("radiobutton", "-command"),
            ("menu", "-command"),
            ("menu", "-postcommand"),
            ("ttk::combobox", "-postcommand"),
            ("spinbox", "-command"), // percent-substitution (%W %s %d)
            ("spinbox", "-validatecommand"), // percent-substitution
            ("entry", "-validatecommand"), // percent-substitution
            ("entry", "-invalidcommand"), // percent-substitution
        ] {
            assert!(
                reg.command_prefixes(widget, &[".w", opt, "cb"]).is_empty(),
                "{widget} {opt} must stay a script, not a command prefix",
            );
        }
    }

    #[test]
    fn commands_naming_a_cmdprefix_declare_a_command_prefix() {
        // Drift guard: any command whose synopsis literally names a `cmdprefix`
        // argument must declare a `CommandPrefix` (static table, resolver, or
        // command-prefix option) so callbacks light up references / call-graph /
        // W123 / arity.  The allowlist holds genuinely-deferred callbacks that
        // still need modelling; it is empty now that the option-value
        // callbacks (`mime::getbody -command`, …) carry `OptionSpec` arrays.
        const DEFERRED_OPTION_PREFIX: &[&str] = &[];
        let reg = CommandRegistry::build_default();
        let mut gaps = Vec::new();
        for name in reg.command_names() {
            if DEFERRED_OPTION_PREFIX.contains(&name) {
                continue;
            }
            let spec = reg.get(name).expect("registered");
            let mut synopses: Vec<&str> = Vec::new();
            if let Some(h) = &spec.hover {
                synopses.extend(h.synopsis.iter().copied());
            }
            synopses.extend(spec.forms.iter().map(|f| f.synopsis));
            synopses.extend(spec.subcommands.iter().map(|s| s.synopsis));
            for syn in synopses {
                if !syn.to_ascii_lowercase().contains("cmdprefix") {
                    continue;
                }
                // Probe with the synopsis words after the command name; a
                // declared prefix yields a non-empty result.  Strip the `?…?`
                // optionality markers so an option-value prefix
                // (`?-command cmdprefix?`) presents its bare `-command` /
                // `cmdprefix` words to the option scanner.
                let args: Vec<&str> = syn
                    .split_whitespace()
                    .skip(1)
                    .map(|w| w.trim_matches('?'))
                    .collect();
                if reg.command_prefixes(name, &args).is_empty() {
                    gaps.push(format!("{name}: {syn}"));
                }
            }
        }
        gaps.sort();
        assert!(
            gaps.is_empty(),
            "commands whose synopsis names a cmdprefix but declare no CommandPrefix:\n{}",
            gaps.join("\n"),
        );
    }

    #[test]
    fn arg_indices_for_role_command_prefix_matches_command_prefixes() {
        // The delegation invariant: `arg_indices_for_role(CommandPrefix)` is
        // exactly the positions `command_prefixes` reports — so highlighting,
        // param-traits, and the call-reference extractor never drift.
        let reg = CommandRegistry::build_default();
        for (name, args) in [
            ("lsort", &["-command", "cmp", "{a b}"][..]),
            ("socket", &["-server", "accept", "9000"][..]),
            ("tcltest::customMatch", &["exact", "cmp"][..]),
            ("selection", &["handle", ".w", "getData"][..]),
            ("regsub", &["-command", "re", "s", "cb"][..]),
            ("namespace", &["unknown", "handler"][..]),
            ("package", &["unknown", "handler"][..]),
            ("coroinject", &["myCoro", "cb", "x"][..]),
        ] {
            let via_role = reg.arg_indices_for_role(name, args, ArgRole::CommandPrefix);
            let via_prefixes: Vec<usize> = reg
                .command_prefixes(name, args)
                .into_iter()
                .map(|(i, _)| i)
                .collect();
            assert_eq!(via_role, via_prefixes, "delegation mismatch for {name}");
        }
    }

    #[test]
    fn namespace_name_option_carries_name_role() {
        // `interp invokehidden -namespace ns cmd` — the `-namespace` value is a
        // symbolic (namespace) name (Phase 7): captured declaratively for a
        // Name query, never for Body/VarWrite (not recursed, not a var def).
        let reg = CommandRegistry::build_default();
        let args = ["invokehidden", "-namespace", "ns", "cmd"];
        assert_eq!(
            reg.arg_indices_for_role("interp", &args, ArgRole::Name),
            vec![2],
            "the -namespace value should carry the Name role",
        );
        assert!(
            reg.arg_indices_for_role("interp", &args, ArgRole::Body)
                .is_empty()
                && reg
                    .arg_indices_for_role("interp", &args, ArgRole::VarWrite)
                    .is_empty(),
            "a name value must not be recursed or treated as a variable",
        );
    }

    #[test]
    fn bind_script_form_recurses_only_the_trailing_script() {
        // `bind $w <KeyPress> {…}` binds a script (issue #785): the third
        // argument is a deferred event-handler body and must be recursed for
        // highlighting.  The `bind tag` / `bind tag sequence` query forms carry
        // no script and must not surface a Body.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role("bind", &["$w", "<KeyPress>", "{…}"], ArgRole::Body),
            vec![2],
            "the trailing script of the three-argument form is a body",
        );
        // The `+script` append form is still the trailing argument.
        assert_eq!(
            reg.arg_indices_for_role("bind", &[".b", "<Enter>", "+{puts hi}"], ArgRole::Body),
            vec![2],
        );
        assert!(
            reg.arg_indices_for_role("bind", &["$w"], ArgRole::Body)
                .is_empty(),
            "the single-tag query form has no script",
        );
        assert!(
            reg.arg_indices_for_role("bind", &["$w", "<KeyPress>"], ArgRole::Body)
                .is_empty(),
            "the tag+sequence query form has no script",
        );
    }

    #[test]
    fn wm_protocol_handler_is_a_body() {
        // `wm protocol . WM_DELETE_WINDOW {script}` registers a deferred
        // handler script as its third argument; the query forms carry none.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "wm",
                &["protocol", ".", "WM_DELETE_WINDOW", "{exit}"],
                ArgRole::Body,
            ),
            // subcommand path offsets by +1 for the `protocol` word.
            vec![3],
            "the wm protocol handler command is a script body",
        );
        assert!(
            reg.arg_indices_for_role("wm", &["protocol", ".", "WM_DELETE_WINDOW"], ArgRole::Body)
                .is_empty(),
            "the two-argument `wm protocol window name` query form has no script",
        );
        assert!(
            reg.arg_indices_for_role("wm", &["protocol", "."], ArgRole::Body)
                .is_empty(),
            "the one-argument `wm protocol window` query form has no script",
        );
    }

    #[test]
    fn canvas_bind_subcommand_script_is_a_body() {
        // `pathName bind tagOrId sequence script` binds a deferred handler.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "canvas",
                &["bind", "item", "<Button>", "{p}"],
                ArgRole::Body
            ),
            vec![3],
            "the canvas bind subcommand's trailing script is a body",
        );
        assert!(
            reg.arg_indices_for_role("canvas", &["bind", "item", "<Button>"], ArgRole::Body)
                .is_empty(),
            "the canvas bind query form has no script",
        );
    }

    #[test]
    fn ttk_widgets_require_tk_8_5() {
        // ttk (themed Tk) widgets were introduced in Tk 8.5, so they must be
        // gated out when only an older Tk is guaranteed by `package require`.
        let reg = CommandRegistry::build_default();
        for name in [
            "ttk::button",
            "ttk::treeview",
            "ttk::notebook",
            "ttk::style",
        ] {
            let spec = reg.get(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(
                !spec.available_for_version(Some("8.4")),
                "{name} must not be available under Tk 8.4",
            );
            assert!(
                spec.available_for_version(Some("8.5")),
                "{name} must be available under Tk 8.5",
            );
            assert!(
                spec.available_for_version(None),
                "{name} must be permissive when no Tk version is pinned",
            );
        }
    }

    #[test]
    fn text_tag_bind_script_is_a_body() {
        // `pathName tag bind tagName sequence script` binds a deferred
        // event-handler script as its trailing word (issue #785 class).
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "text",
                &["tag", "bind", "sel", "<Key>", "{p}"],
                ArgRole::Body,
            ),
            // `tag` subcommand offset (+1) plus index 3 within the tag args.
            vec![4],
            "the text tag bind trailing script is a body",
        );
        assert!(
            reg.arg_indices_for_role("text", &["tag", "add", "sel", "1.0", "end"], ArgRole::Body)
                .is_empty(),
            "non-bind tag subcommands carry no script",
        );
    }

    #[test]
    fn selection_handle_command_is_a_command_prefix() {
        // `selection handle window command` — the trailing command is a
        // prefix Tk appends offset/maxChars to, not a recursed script body.
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.arg_indices_for_role(
                "selection",
                &["handle", ".w", "getData"],
                ArgRole::CommandPrefix
            ),
            vec![2],
            "the selection handle command prefix is captured",
        );
        assert!(
            reg.arg_indices_for_role("selection", &["handle", ".w", "getData"], ArgRole::Body)
                .is_empty(),
            "a command prefix is not recursed as a script body",
        );
        // With leading option/value pairs the command is still the last arg.
        assert_eq!(
            reg.arg_indices_for_role(
                "selection",
                &["handle", "-format", "STRING", ".w", "getData"],
                ArgRole::CommandPrefix,
            ),
            vec![4],
        );
    }

    #[test]
    fn writes_first_arg_variable_membership() {
        let reg = CommandRegistry::build_default();
        // TP: the five first-arg writers.
        for cmd in ["set", "append", "lappend", "incr", "lset"] {
            assert!(reg.writes_first_arg_variable(cmd), "{cmd} writes arg 0");
        }
        // FP guards: `unset` destroys (not a write); value-taking and
        // unknown commands are out.
        for cmd in ["unset", "puts", "llength", "foreach", "nosuchcmd"] {
            assert!(!reg.writes_first_arg_variable(cmd), "{cmd} must not match");
        }
    }

    #[test]
    fn rmw_first_arg_variable_membership() {
        let reg = CommandRegistry::build_default();
        // TP: read-modify-write commands fold the current value in.
        for cmd in ["append", "lappend", "incr", "lset"] {
            assert!(reg.rmw_first_arg_variable(cmd), "{cmd} is RMW");
        }
        // FP guards: a whole-value `set` is rename-safe; `unset` destroys.
        for cmd in ["set", "unset", "puts", "nosuchcmd"] {
            assert!(!reg.rmw_first_arg_variable(cmd), "{cmd} must not match");
        }
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
            // Tcl 9.0+: shares `lreplace`'s core (see
            // `tcl-vm/src/cmd_list.rs::cmd_ledit`); a flat single-variable
            // read-modify-write with no eval fallback and no sublist-index
            // descent (unlike `lset`, which deliberately stays off this
            // list) — see `commands/tcl/ledit.rs`.
            "ledit",
            "lrepeat",
            "lassign",
            // Tcl 9.0+: a flat native dispatch over `tcl-cmd-core::lseq`
            // (`tcl-vm/src/cmd_lseq.rs::cmd_lseq`) with no script-body eval
            // fallback of its own — its only recursive edge is evaluating
            // one argument word as an *expression* via `Vm::eval_expr`,
            // the same evaluator `expr` (also on this list) runs on, not a
            // general eval-a-script fallback — see `commands/tcl/lseq.rs`.
            "lseq",
            "concat",
            "split",
            "join",
            "string",
            "format",
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
            // `throw type message` (Tcl 8.6+) — `cmd_throw`
            // (`tcl-vm/src/cmd_try.rs`) takes its two already-substituted
            // arguments, validates `type` as a non-empty Tcl list, and
            // builds its return-options dict directly with no eval
            // fallback of any kind (it never touches `vm`), structurally
            // identical to `cmd_error` in this respect. See
            // `commands/tcl/throw_.rs`.
            "throw",
            // `::tcl::unsupported::corotype coroName` (and its
            // namespace-relative `tcl::unsupported::corotype` spelling) — a
            // flat lookup into the coroutine table
            // (`tcl-vm/src/cmd_coro.rs::cmd_corotype`): no eval fallback, no
            // `Frame` of its own. Both spellings share the same `make_spec`
            // and so the same traits. See
            // `commands/tcl/tcl_unsupported_corotype.rs`.
            "::tcl::unsupported::corotype",
            "tcl::unsupported::corotype",
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

    /// `unset x y z` names *every* argument as a variable (issue #774), not
    /// just the first, so all of them highlight as variables.
    #[test]
    fn unset_marks_every_name() {
        let reg = CommandRegistry::build_default();
        let vars = reg.arg_indices_for_role("unset", &["x", "y", "z"], ArgRole::VarWrite);
        assert_eq!(vars, vec![0, 1, 2]);
    }

    /// `unset -nocomplain -- a b` skips the leading options and names only the
    /// real variables (`a`, `b`), mirroring `lower_unset`.
    #[test]
    fn unset_skips_leading_options() {
        let reg = CommandRegistry::build_default();
        let vars =
            reg.arg_indices_for_role("unset", &["-nocomplain", "--", "a", "b"], ArgRole::VarWrite);
        assert_eq!(vars, vec![2, 3]);
    }

    /// `unset` recognises only `-nocomplain` / `--` as options — a dash-prefixed
    /// word like `-foo` is a real variable name (verified against tclsh), so it
    /// keeps its `VarWrite` role.
    #[test]
    fn unset_dash_name_is_a_variable() {
        let reg = CommandRegistry::build_default();
        // `unset -foo bar` — both are variables (no `--` needed to reach them).
        assert_eq!(
            reg.arg_indices_for_role("unset", &["-foo", "bar"], ArgRole::VarWrite),
            vec![0, 1]
        );
        // `unset -nocomplain -foo` — `-nocomplain` is skipped, `-foo` is a name.
        assert_eq!(
            reg.arg_indices_for_role("unset", &["-nocomplain", "-foo"], ArgRole::VarWrite),
            vec![1]
        );
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

    /// `trace add command`/`execution` trace a command *by name*, so the name
    /// argument is a `CommandName` reference (navigation follows it), while a
    /// `variable` trace's name is not.
    #[test]
    fn arg_indices_for_role_trace_add_command_name_reference() {
        let reg = CommandRegistry::build_default();
        for kind in ["command", "execution"] {
            let names = reg.arg_indices_for_role(
                "trace",
                &["add", kind, "foo", "enter", "body"],
                ArgRole::CommandName,
            );
            // +1 subcommand offset: `foo` is at full-args index 2.
            assert!(names.contains(&2), "{kind}: CommandName names={names:?}");
        }
        let var = reg.arg_indices_for_role(
            "trace",
            &["add", "variable", "x", "write", "body"],
            ArgRole::CommandName,
        );
        assert!(var.is_empty(), "a variable trace names no command: {var:?}");
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

    /// `plain_body_arg_indices` surfaces the same-frame control-flow /
    /// `eval` bodies a nested dispatch (e.g. `TclOO` `my method`) still
    /// executes inside — every `Plain`-kind `ArgRole::Body` argument.
    #[test]
    fn plain_body_arg_indices_covers_same_frame_bodies() {
        let reg = CommandRegistry::build_default();
        assert_eq!(reg.plain_body_arg_indices("if", &["1", "{body}"]), vec![1]);
        assert_eq!(
            reg.plain_body_arg_indices("while", &["1", "{body}"]),
            vec![1]
        );
        assert_eq!(
            reg.plain_body_arg_indices("foreach", &["v", "$list", "{body}"]),
            vec![2]
        );
        assert_eq!(reg.plain_body_arg_indices("eval", &["{body}"]), vec![0]);
        assert_eq!(
            reg.plain_body_arg_indices("catch", &["{body}", "res"]),
            vec![0]
        );
    }

    /// `Structural` bodies (`proc`, `oo::class create`, `uplevel`,
    /// `namespace eval`) never come back from `plain_body_arg_indices` —
    /// they run in a definition / different-frame context, so a dispatch
    /// found inside is not still the caller's scope.
    #[test]
    fn plain_body_arg_indices_excludes_structural_bodies() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.plain_body_arg_indices("proc", &["name", "args", "{body}"]),
            Vec::<usize>::new()
        );
        assert_eq!(
            reg.plain_body_arg_indices("uplevel", &["1", "{body}"]),
            Vec::<usize>::new()
        );
        assert_eq!(
            reg.plain_body_arg_indices("namespace", &["eval", "ns", "{body}"]),
            Vec::<usize>::new()
        );
    }

    /// An unknown command name (a user proc, a `TclOO` method body's own
    /// `unknownProc arg` call) has no registry spec at all — returns empty
    /// rather than panicking.
    #[test]
    fn plain_body_arg_indices_unknown_command_is_empty() {
        let reg = CommandRegistry::build_default();
        assert_eq!(
            reg.plain_body_arg_indices("myUserProc", &["a", "b"]),
            Vec::<usize>::new()
        );
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
        reg.load_eda_packs("synopsys-eda-tcl");
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
        // ``-start`` takes a value; ``-nocase`` does not.
        // ``-start`` takes a value; ``-nocase`` does not.  The
        // resolver returns the borrowed options slice; callers
        // filter via ``OptionSpec::takes_value``.
        assert!(
            profile
                .options
                .iter()
                .any(|o| o.name == "-start" && o.takes_value())
        );
        assert!(
            profile
                .options
                .iter()
                .any(|o| o.name == "-nocase" && !o.takes_value())
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
        assert!(!noserver.takes_value());
        let version = respond
            .options
            .iter()
            .find(|o| o.name == "-version")
            .unwrap();
        assert!(version.takes_value());

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

    /// `unit_linkage` composes `spec.traits | sub.traits` and filters to the
    /// linkage union, so the answer is subcommand-precise: `package provide`
    /// publishes an API surface, `package require` pulls another unit in, and
    /// `package names` does neither (issue #977).
    #[test]
    fn unit_linkage_is_subcommand_precise() {
        let reg = CommandRegistry::build_default();
        let empty = DialectSet::empty();
        assert_eq!(
            reg.unit_linkage("package", &["provide", "mylib", "1.0"], empty),
            Traits::PROVIDES_PACKAGE
        );
        assert_eq!(
            reg.unit_linkage("package", &["ifneeded", "mylib", "1.0", "body"], empty),
            Traits::PROVIDES_PACKAGE
        );
        assert_eq!(
            reg.unit_linkage("package", &["require", "mylib"], empty),
            Traits::LOADS_EXTERNAL_UNIT
        );
        assert_eq!(
            reg.unit_linkage("package", &["names"], empty),
            Traits::empty()
        );
    }

    /// `invocation_traits` composes `spec.traits | sub.traits` for the
    /// concrete call, which is the only way the eval-family bits on the
    /// compound members are visible: `namespace eval` / `namespace inscope` /
    /// `interp eval` carry `EVALUATES_CODE` on the **subcommand**, so a
    /// parent-only `get(name).traits` test misses them (issue #1055).
    #[test]
    fn invocation_traits_compose_subcommand_traits() {
        let reg = CommandRegistry::build_default();
        let empty = DialectSet::empty();
        // TP — the eval family, bare and compound.
        for (name, args) in [
            ("eval", &["{set x 1}"][..]),
            ("uplevel", &["1", "{set x 1}"][..]),
            ("namespace", &["eval", "ns", "{set x 1}"][..]),
            ("namespace", &["inscope", "ns", "{set x 1}"][..]),
            ("interp", &["eval", "slave", "{set x 1}"][..]),
        ] {
            let traits = reg.invocation_traits(name, args, empty);
            assert!(
                traits.contains(Traits::EVALUATES_CODE),
                "{name} {args:?} must carry EVALUATES_CODE"
            );
            assert!(
                traits.contains(Traits::SCRIPT_CONCATENATES_ARGS),
                "{name} {args:?} must carry SCRIPT_CONCATENATES_ARGS"
            );
        }
        // TN — the parent spec alone carries neither bit, which is exactly
        // why composing is required.
        let parent = reg.get("namespace").expect("namespace spec").traits;
        assert!(!parent.contains(Traits::EVALUATES_CODE));
        // TN — a sibling subcommand that evaluates nothing stays clean.
        assert!(
            !reg.invocation_traits("namespace", &["delete", "ns"], empty)
                .contains(Traits::EVALUATES_CODE)
        );
        // TN — an unknown command carries no traits at all.
        assert_eq!(
            reg.invocation_traits("no_such_command_xyz", &["a"], empty),
            Traits::empty()
        );
        // The parent's own traits still compose in — a subcommand's traits
        // are additive, not a replacement.
        assert!(
            reg.invocation_traits("namespace", &["eval", "ns", "{}"], empty)
                .contains(Traits::LANGUAGE_KEYWORD),
            "the parent `namespace` spec's own bits survive composition"
        );
    }

    /// The whole registry-declared boundary surface, resolved by name: every
    /// command that widens a file's caller set reports its kind, a
    /// `::`-qualified spelling resolves the same, and a command that does
    /// neither reports nothing.
    #[test]
    fn unit_linkage_covers_every_declared_boundary_command() {
        let reg = CommandRegistry::build_default();
        let empty = DialectSet::empty();
        for (name, args, want) in [
            ("source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("::source", &["lib.tcl"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("load", &["libx.so"][..], Traits::LOADS_EXTERNAL_UNIT),
            ("auto_load", &["helper"][..], Traits::LOADS_EXTERNAL_UNIT),
            (
                "auto_import",
                &["::lib::*"][..],
                Traits::LOADS_EXTERNAL_UNIT,
            ),
            // `namespace import` is deliberately not a boundary — see its
            // own spec comment.
            (
                "namespace",
                &["import", "::lib::helper"][..],
                Traits::empty(),
            ),
            (
                "namespace",
                &["export", "helper"][..],
                Traits::EXPORTS_COMMAND,
            ),
            (
                "namespace",
                &["ensemble", "create"][..],
                Traits::EXPORTS_COMMAND,
            ),
            ("namespace", &["eval", "::ns", "{}"][..], Traits::empty()),
            ("set", &["x", "1"][..], Traits::empty()),
            ("no_such_command_xyz", &["source"][..], Traits::empty()),
        ] {
            assert_eq!(
                reg.unit_linkage(name, args, empty),
                want,
                "unit_linkage({name}, {args:?})"
            );
        }
    }

    /// Widening `Traits` to `u128` gave `SAFE_INTERP_HIDDEN` a bit of its own
    /// (issue #1031): while it aliased `TRANSFERS_CONTROL` at bit 61, every
    /// `break`/`continue`/`tailcall`/`yield` read as safe-interp-hidden and
    /// every `cd`/`exec`/`glob`/… read as control-transferring, which
    /// `FRAME_SENSITIVE_TRAITS` consumes directly.
    #[test]
    fn safe_interp_hidden_no_longer_aliases_transfers_control() {
        let reg = CommandRegistry::build_default();
        assert_ne!(Traits::TRANSFERS_CONTROL, Traits::SAFE_INTERP_HIDDEN);
        let cd = reg.get("cd").expect("cd is registered");
        assert!(cd.traits.contains(Traits::SAFE_INTERP_HIDDEN));
        assert!(!cd.traits.contains(Traits::TRANSFERS_CONTROL));
        assert!(!reg.is_frame_sensitive("cd"));
        let brk = reg.get("break").expect("break is registered");
        assert!(brk.traits.contains(Traits::TRANSFERS_CONTROL));
        assert!(!brk.traits.contains(Traits::SAFE_INTERP_HIDDEN));
    }
}
