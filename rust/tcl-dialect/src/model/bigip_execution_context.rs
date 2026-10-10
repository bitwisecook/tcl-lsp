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

//! BIG-IP execution contexts key runtime facts, command surfaces and storage.
//!
//! TMM, tmsh, iApp implementation and iCall agree on the measured F5 grammar cases,
//! while each retains its own environment and evidence. Host Tcl is an
//! independent control. APL and its Tcl callbacks are unmeasured and cannot
//! inherit facts from implementation scripts. A measured context can still
//! have an unknown build profile or no registered environment.

use super::family::{BuildProfileId, CoreProfileId, Family, Release};

/// A BIG-IP-relevant language or interpreter execution context.
///
/// The spellings are the transcript's own labels
/// (`TCLLSPPROBE|TmmIRule|…` in
/// `scripts/dev/bigip-probes/results/10-context-parity.txt`), so a row in
/// this repository and a line in an appliance transcript name the same
/// thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BigIpExecutionContext {
    /// An iRule running in TMM. Measured: patchlevel 8.4.6, 152 commands,
    /// fabricated `tcl_platform` (`os BIG-IP`, `machine` = hostname,
    /// `tmmVersion 26`, `wordSize 8`), **no** `exec`, and command
    /// resolution at **rule load** even inside `catch` (§4a).
    TmmIRule,
    /// A tmsh `cli script` run through `script::run`. Measured: patchlevel
    /// 8.4.6 with `tcl_patchLevel` **unset**, 95 commands, `tcl_platform`
    /// **empty**, working `exec`, and a non-standard `info vartype`
    /// subcommand (§4a).
    TmshCliScript,
    /// The Tcl in an iApp template's `implementation` field, run by
    /// `scriptd`. Measured: patchlevel 8.4.6, 95 commands, a real-ish
    /// Linux `tcl_platform` with **`wordSize 4`** — a 32-bit build of the
    /// same trunk — working `exec`, and a large ambient package set
    /// (§4/§4a).
    IAppImplementation,
    /// An iCall script entered by a triggered handler through scriptd.
    /// Its own transcript reports Tcl 8.4.6, 95 commands and working `exec`.
    /// Interpreter width is not measured by that transcript.
    ICallScript,
    /// The APL presentation language in an iApp template's `presentation`
    /// field. **Never measured** (E4 step 6). APL is a presentation DSL
    /// that *contains* Tcl, not a Tcl dialect: its keywords must never
    /// become Tcl commands.
    IAppPresentationApl,
    /// A Tcl callback nested inside APL (`choice … tcl { … }`). **Never
    /// measured** (E4 step 6). It must not be assumed equivalent to
    /// implementation Tcl merely because both are spelled in Tcl.
    IAppPresentationTclCallback,
    /// The appliance's own `/usr/bin/tclsh`. Provenance only — it is
    /// **not** a BIG-IP execution context, and the run proved why:
    /// `tclsh8.4` on the same box is **8.4.13**, not the 8.4.6 embedded in
    /// the measured embedded F5 contexts. Host versions cannot identify
    /// an embedded interpreter.
    HostShellTcl,
}

/// Whether a context has an appliance transcript behind it, and why not
/// when it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextMeasurement {
    /// Exercised on a live appliance; the Registry F5 evidence inventory has rows.
    Measured,
    /// Never exercised. The prose is the driver's own reason, and it is
    /// the only honest answer for this context — no other context's row
    /// may be substituted for it.
    Unmeasured(&'static str),
}

impl ContextMeasurement {
    /// Whether an appliance transcript backs this context.
    #[must_use]
    pub const fn is_measured(self) -> bool {
        matches!(self, Self::Measured)
    }
}

impl BigIpExecutionContext {
    /// Every independently identified execution context.
    pub const ALL: [Self; 7] = [
        Self::TmmIRule,
        Self::TmshCliScript,
        Self::IAppImplementation,
        Self::ICallScript,
        Self::IAppPresentationApl,
        Self::IAppPresentationTclCallback,
        Self::HostShellTcl,
    ];

    /// The transcript's own label for the context.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TmmIRule => "TmmIRule",
            Self::TmshCliScript => "TmshCliScript",
            Self::IAppImplementation => "IAppImplementation",
            Self::ICallScript => "ICallScript",
            Self::IAppPresentationApl => "IAppPresentationApl",
            Self::IAppPresentationTclCallback => "IAppPresentationTclCallback",
            Self::HostShellTcl => "HostShellTcl",
        }
    }

    /// Whether the context executes **on the appliance as a BIG-IP
    /// feature**. False for [`Self::HostShellTcl`], which is an ordinary
    /// system `tclsh` that happens to be installed there (§4a: it is a
    /// different Tcl build entirely).
    #[must_use]
    pub const fn is_appliance_hosted(self) -> bool {
        !matches!(self, Self::HostShellTcl)
    }

    /// Whether an appliance transcript backs the context. APL contexts are unmeasured.
    #[must_use]
    pub const fn measurement(self) -> ContextMeasurement {
        match self {
            Self::TmmIRule
            | Self::TmshCliScript
            | Self::IAppImplementation
            | Self::ICallScript
            | Self::HostShellTcl => ContextMeasurement::Measured,
            Self::IAppPresentationApl | Self::IAppPresentationTclCallback => {
                ContextMeasurement::Unmeasured("no non-interactive presentation renderer exercised")
            }
        }
    }

    /// Whether the context's language is Tcl at all.
    ///
    /// [`Self::IAppPresentationApl`] is the one that is not: APL is a
    /// presentation DSL with `define`/`section`/`choice`/`optional`
    /// clauses that embeds Tcl. APL ranges cannot be routed into the Tcl
    /// command registry.
    #[must_use]
    pub const fn is_tcl(self) -> bool {
        !matches!(self, Self::IAppPresentationApl)
    }

    /// The core-language family this context's Tcl belongs to, or `None`
    /// when the context is unmeasured or is not Tcl.
    ///
    /// `None` is load-bearing: an unmeasured context must not inherit
    /// [`Family::F5Tcl`] from `IAppImplementation` just because both live
    /// inside an iApp template.
    #[must_use]
    pub const fn family(self) -> Option<Family> {
        match self {
            Self::TmmIRule => Some(Family::F5Irules),
            Self::TmshCliScript | Self::IAppImplementation | Self::ICallScript => {
                Some(Family::F5Tcl)
            }
            Self::HostShellTcl => Some(Family::Tcl),
            Self::IAppPresentationApl | Self::IAppPresentationTclCallback => None,
        }
    }

    /// Hosted source context selected by an actual environment identity.
    /// A compatibility family, profile label or Tcl release is insufficient;
    /// iCall has no registered environment and needs an explicit context.
    #[must_use]
    pub fn for_environment(id: &super::environment::EnvironmentId) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|context| context.environment_name() == Some(id.as_str()))
    }

    /// The independently measured build profile of the context's interpreter.
    ///
    /// [`BuildProfileId::F5Scriptd32`] for `IAppImplementation` — measured
    /// `tcl_platform(wordSize) == 4` against TMM's 8 (§4) — and
    /// [`BuildProfileId::Unknown`] where interpreter width is unmeasured,
    /// including tmsh and iCall. Capability queries preserve that uncertainty.
    #[must_use]
    pub const fn build_profile(self) -> BuildProfileId {
        match self {
            Self::TmmIRule | Self::HostShellTcl => BuildProfileId::Canonical,
            Self::IAppImplementation => BuildProfileId::F5Scriptd32,
            Self::IAppPresentationApl
            | Self::IAppPresentationTclCallback
            | Self::ICallScript
            | Self::TmshCliScript => BuildProfileId::Unknown,
        }
    }

    /// The canonical environment name this context selects, or `None` when
    /// it has no environment of its own.
    ///
    /// [`Self::HostShellTcl`] deliberately has none: it is provenance, and
    /// giving it an environment would invite exactly the substitution §4a
    /// warns about.
    #[must_use]
    pub const fn environment_name(self) -> Option<&'static str> {
        match self {
            Self::TmmIRule => Some("f5-irules"),
            Self::TmshCliScript => Some("f5-tmsh"),
            Self::IAppImplementation => Some("f5-iapps"),
            Self::IAppPresentationApl
            | Self::IAppPresentationTclCallback
            | Self::HostShellTcl
            | Self::ICallScript => None,
        }
    }

    /// The core profile identity of the context's interpreter, or `None`
    /// when the context is unmeasured or not Tcl.
    #[must_use]
    pub fn core_profile(self) -> Option<CoreProfileId> {
        let release = match self.family()? {
            Family::F5Irules => Release::F5_IRULES_TMM,
            Family::F5Tcl => Release::F5_TCL_TMOS,
            // The host binary is a plain 8.4/8.5 build; the 8.4 ladder
            // step is the one the transcript's `tclsh8.4` control ran on.
            Family::Tcl => Release::TCL_8_4,
            Family::Jim => return None,
        };
        Some(CoreProfileId::new(release, self.build_profile()))
    }

    /// Whether a fact measured in `self` may be read as a fact about
    /// `other`.
    ///
    /// Always `false` for distinct contexts: *"A command-availability fact measured in one context must
    /// never be promoted to another"* (§4a) — `exec` works in a `cli
    /// script` and an iApp implementation and is absent in TMM, which is
    /// the concrete case behind it.
    #[must_use]
    pub fn promotes_facts_to(self, other: Self) -> bool {
        self == other
    }
}

impl std::fmt::Display for BigIpExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
