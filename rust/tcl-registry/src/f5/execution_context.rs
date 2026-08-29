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

//! [`BigIpExecutionContext`] — the F1 key.
//!
//! The evidence review's executive finding is that the F5 model needs
//! another key before it can be ratified, because six distinct language
//! contexts were being treated as one: they have different command
//! surfaces, variables, package paths, security policies, lifetimes, and
//! execution engines. This is that key.
//!
//! The live run
//! (`docs/design/bigip-irule-parser-measurements.md` §4a) then *refined*
//! F1 in one important way: the three BIG-IP-hosted Tcl contexts are **one
//! parser** — every grammar and newline case in a single 34-case list is
//! byte-identical across `TmmIRule`, `TmshCliScript` and
//! `IAppImplementation` — but they are emphatically **not one
//! environment**. `exec` is absent in TMM and works in the other two,
//! `llength [info commands]` counts 152/95/95, `tcl_platform` is
//! fabricated / **empty** / real-Linux respectively, and `tcl_patchLevel`
//! does not exist at all in a `cli script`. So the key splits **command
//! surface, variables, policy and evidence**, not grammar; grammar answers
//! from the `f5-tcl` trunk family for all three.
//!
//! Two of the six were never exercised. [`BigIpExecutionContext::IAppPresentationApl`]
//! and [`BigIpExecutionContext::IAppPresentationTclCallback`] are recorded
//! [`ContextMeasurement::Unmeasured`] by the driver itself (E4 step 6:
//! *"Otherwise record that context as `Unknown`; never copy the
//! implementation result into it"*), and this module is where that
//! discipline is mechanised: an unmeasured context has no family, no build
//! profile, no environment and no core profile, and nothing in
//! [`crate::f5::evidence`] will hand it another context's row.

use tcl_dialect::model::family::{BuildProfileId, CoreProfileId, Family, Release};

/// One of the six BIG-IP-relevant Tcl execution contexts (F1).
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
    /// all three F5 contexts (§4a). Reading a version off the host would
    /// have been wrong for every F5 row.
    HostShellTcl,
}

/// Whether a context has an appliance transcript behind it, and why not
/// when it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextMeasurement {
    /// Exercised on a live appliance; [`crate::f5::evidence`] has rows.
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
    /// Every context, in the review's declaration order.
    pub const ALL: [Self; 6] = [
        Self::TmmIRule,
        Self::TmshCliScript,
        Self::IAppImplementation,
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

    /// Whether an appliance transcript backs the context (§11: four of the
    /// six were measured; the two APL contexts were not).
    #[must_use]
    pub const fn measurement(self) -> ContextMeasurement {
        match self {
            Self::TmmIRule
            | Self::TmshCliScript
            | Self::IAppImplementation
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
    /// clauses that merely *embeds* Tcl. A consumer that routes an APL
    /// range into the Tcl registry has made the F1 mistake.
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
            Self::TmshCliScript | Self::IAppImplementation => Some(Family::F5Tcl),
            Self::HostShellTcl => Some(Family::Tcl),
            Self::IAppPresentationApl | Self::IAppPresentationTclCallback => None,
        }
    }

    /// The build profile of the context's interpreter (review B1).
    ///
    /// [`BuildProfileId::F5Scriptd32`] for `IAppImplementation` — measured
    /// `tcl_platform(wordSize) == 4` against TMM's 8 (§4) — and
    /// [`BuildProfileId::Unknown`] for every unmeasured context, so
    /// capability queries answer `Unknown` rather than a canonical
    /// default.
    #[must_use]
    pub const fn build_profile(self) -> BuildProfileId {
        match self {
            Self::TmmIRule | Self::TmshCliScript | Self::HostShellTcl => BuildProfileId::Canonical,
            Self::IAppImplementation => BuildProfileId::F5Scriptd32,
            Self::IAppPresentationApl | Self::IAppPresentationTclCallback => {
                BuildProfileId::Unknown
            }
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
            Self::IAppPresentationApl | Self::IAppPresentationTclCallback | Self::HostShellTcl => {
                None
            }
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
    /// Always `false` for distinct contexts. This is the F1/F4 rule stated
    /// as code: *"A command-availability fact measured in one context must
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

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{Family};
    use super::*;
    use tcl_dialect::model::family::CapabilityAnswer;

    #[test]
    fn the_six_contexts_are_distinct_and_named_as_the_transcript_names_them() {
        let mut seen = std::collections::HashSet::new();
        for context in BigIpExecutionContext::ALL {
            assert!(seen.insert(context.as_str()), "{context}: duplicate label");
            assert_eq!(context.to_string(), context.as_str());
        }
        assert_eq!(seen.len(), 6);
    }

    /// §11: four contexts measured, two `Unknown` — and the two unmeasured
    /// ones stay unknown all the way down. No family, no environment, no
    /// core profile, and a build profile whose capability answers are
    /// `Unknown` rather than the canonical column.
    #[test]
    fn unmeasured_contexts_stay_unknown_all_the_way_down() {
        let measured: Vec<_> = BigIpExecutionContext::ALL
            .into_iter()
            .filter(|c| c.measurement().is_measured())
            .collect();
        assert_eq!(
            measured,
            vec![
                BigIpExecutionContext::TmmIRule,
                BigIpExecutionContext::TmshCliScript,
                BigIpExecutionContext::IAppImplementation,
                BigIpExecutionContext::HostShellTcl,
            ]
        );

        for context in [
            BigIpExecutionContext::IAppPresentationApl,
            BigIpExecutionContext::IAppPresentationTclCallback,
        ] {
            assert!(!context.measurement().is_measured(), "{context}");
            assert_eq!(context.family(), None, "{context}");
            assert_eq!(context.environment_name(), None, "{context}");
            assert_eq!(context.core_profile(), None, "{context}");
            assert_eq!(
                context.build_profile(),
                BuildProfileId::Unknown,
                "{context}"
            );
        }

        // The APL contexts sit inside the same template as
        // `IAppImplementation`; that proximity must not leak its row.
        let implementation = BigIpExecutionContext::IAppImplementation;
        assert_eq!(implementation.family(), Some(Family::F5Tcl));
        assert_eq!(implementation.environment_name(), Some("f5-iapps"));
        for apl in [
            BigIpExecutionContext::IAppPresentationApl,
            BigIpExecutionContext::IAppPresentationTclCallback,
        ] {
            assert!(!implementation.promotes_facts_to(apl));
            assert!(!apl.promotes_facts_to(implementation));
        }
    }

    /// §4a's three consequences, as type facts: the three F5 contexts
    /// share one parser but not one environment, the host `tclsh` is not
    /// an F5 context at all, and no fact crosses a context boundary.
    #[test]
    fn contexts_key_surface_and_environment_not_grammar() {
        let irule = BigIpExecutionContext::TmmIRule;
        let tmsh = BigIpExecutionContext::TmshCliScript;
        let iapp = BigIpExecutionContext::IAppImplementation;

        // One parser: the iRules offshoot inherits the trunk grammar
        // whole, so every lexical axis agrees across the three.
        let grammar_of = |context: BigIpExecutionContext| {
            let id = context.core_profile().expect("measured context");
            tcl_dialect::model::family::grammar(id.family(), id.release)
        };
        assert_eq!(grammar_of(irule), grammar_of(tmsh));
        assert_eq!(grammar_of(tmsh), grammar_of(iapp));
        // …and every one of them differs from the host build.
        assert_ne!(
            grammar_of(irule),
            grammar_of(BigIpExecutionContext::HostShellTcl)
        );

        // Not one environment: distinct environments, and the iApp host
        // is a 32-bit build of the same trunk (`wordSize 4`, §4).
        assert_eq!(irule.environment_name(), Some("f5-irules"));
        assert_eq!(tmsh.environment_name(), Some("f5-tmsh"));
        assert_eq!(iapp.environment_name(), Some("f5-iapps"));
        assert_eq!(iapp.build_profile(), BuildProfileId::F5Scriptd32);
        assert_eq!(
            iapp.core_profile()
                .expect("measured")
                .resolve()
                .capabilities
                .word_size_64,
            CapabilityAnswer::No
        );
        assert_eq!(
            tmsh.core_profile()
                .expect("measured")
                .resolve()
                .capabilities
                .word_size_64,
            CapabilityAnswer::Yes
        );

        // The host `tclsh` is provenance only.
        let host = BigIpExecutionContext::HostShellTcl;
        assert!(!host.is_appliance_hosted());
        assert_eq!(host.environment_name(), None);
        assert_eq!(host.family(), Some(Family::Tcl));

        // No fact ever crosses a context boundary (F1/F4).
        for a in BigIpExecutionContext::ALL {
            for b in BigIpExecutionContext::ALL {
                assert_eq!(a.promotes_facts_to(b), a == b, "{a} -> {b}");
            }
        }
    }

    /// APL is a presentation DSL, not a Tcl dialect (F1's required
    /// change): its keywords must never become Tcl commands. The Tcl
    /// callback nested *inside* it is Tcl — but an unmeasured one.
    #[test]
    fn apl_is_not_tcl_but_its_callback_is() {
        assert!(!BigIpExecutionContext::IAppPresentationApl.is_tcl());
        assert!(BigIpExecutionContext::IAppPresentationTclCallback.is_tcl());
        for context in BigIpExecutionContext::ALL {
            if context != BigIpExecutionContext::IAppPresentationApl {
                assert!(context.is_tcl(), "{context}");
            }
        }
    }
}
