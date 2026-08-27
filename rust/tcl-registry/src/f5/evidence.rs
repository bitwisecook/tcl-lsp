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

//! [`EmbeddedRuntimeEvidence`] — F2's truth records.
//!
//! The review's objection was that the shipping model stated *"iRules
//! embeds a genuine Tcl 8.4.6 forever"*, *"iApps runs a real Tcl 8.5.13
//! host"*, and *"tmsh uses Tcl 8.5"* as total facts with no build
//! manifest, source package, binary dependency, appliance transcript, or
//! version matrix behind any of them. Two of those three were then
//! **measured and falsified**: `TmshCliScript` and `IAppImplementation`
//! both report `8.4.6` and both fail every 8.4/8.5 discriminator as 8.4
//! (`docs/design/bigip-irule-parser-measurements.md` §4/§4a). The 8.5.13
//! that the model had adopted turned out to be `/usr/bin/tclsh` — the host
//! binary, unrelated to any F5 execution context.
//!
//! So a fact here is never a bare field. It is a
//! `(context, build, fact, provenance)` record:
//!
//! - **context** — [`BigIpExecutionContext`], because `exec` exists in two
//!   of the three F5 contexts and not the third (§4a);
//! - **build** — [`BigIpBuild`], the exact appliance release *and* build
//!   number the observation was taken on, because one build can falsify a
//!   universal claim but cannot justify "forever";
//! - **fact** — a typed [`RuntimeFact`], because `info patchlevel` alone
//!   is not a semantic profile: F5 can patch parser and command behaviour
//!   without moving that string, which is exactly what happened;
//! - **provenance** — [`EvidenceProvenance`], naming the probe set, the
//!   measurements section, and whether the run met the §E4 contract.
//!
//! ## The resolution rule
//!
//! Unmeasured builds must not silently inherit a measured one. The API is
//! split in two, deliberately with different names and different return
//! types (redesign H5, the assistance/semantics split):
//!
//! - [`measured_fact`] is the **semantic** door. Exact context, exact
//!   build, or `None`. Compiler and analyser hooks that assert something
//!   about a program use this one.
//! - [`assistance_fact`] is the **assistance** door. It may answer with an
//!   explicitly labelled [`EvidenceResolution::NearestKnownAssistance`]
//!   carrying the build it was actually measured on, so a hover or
//!   completion can say *"measured on 21.1.0.1"* rather than implying the
//!   user's build was probed.
//!
//! Neither door crosses a context boundary, ever: assistance widens along
//! the **build** axis only. An `IAppPresentationApl` query answers
//! [`EvidenceResolution::Unknown`] no matter how much is known about
//! `IAppImplementation`.

use tcl_dialect::compare_versions;

use crate::f5::execution_context::BigIpExecutionContext;
use crate::irules_policy::{IRULES_COMPILER_REFUSED, IRULES_INTERPRETER_ABSENT};

/// One appliance software build: the release train plus the build number
/// under it (`21.1.0.1` build `0.0.26`).
///
/// Versions on this type live on [`tcl_dialect::model::VersionAxisId::big_ip`]
/// — never on a Tcl core axis (F6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BigIpBuild {
    /// The BIG-IP software release (`"21.1.0.1"`).
    pub release: &'static str,
    /// The build number under that release (`"0.0.26"`).
    pub build: &'static str,
}

impl BigIpBuild {
    /// The single measured build: BIG-IP 21.1.0.1, build 0.0.26, probed
    /// 2026-08-26.
    pub const MEASURED_21_1_0_1: Self = Self {
        release: "21.1.0.1",
        build: "0.0.26",
    };
}

impl std::fmt::Display for BigIpBuild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} build {}", self.release, self.build)
    }
}

/// Every build this repository has evidence for. One row today — and the
/// review's acceptance matrix wants three (this build, one supported 17.x,
/// one older).
pub const MEASURED_BUILDS: &[BigIpBuild] = &[BigIpBuild::MEASURED_21_1_0_1];

/// The identity of one checked-in probe set under
/// `scripts/dev/bigip-probes/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProbeSetId(pub &'static str);

impl ProbeSetId {
    /// The four-context parity probe (`suites/10-context-parity.cases`,
    /// `results/10-context-parity.txt`) — E4-conforming.
    pub const CONTEXT_PARITY: Self = Self("10-context-parity");
    /// The proc-semantics + disabled-command-mechanism run
    /// (`suites/11-proc-semantics.cases`, `results/11-proc-semantics.txt`).
    pub const PROC_SEMANTICS: Self = Self("11-proc-semantics");
    /// The 25 Tcl 8.5-feature cases (`suites/09-tcl85-features.tcl`).
    pub const RELEASE_FEATURES: Self = Self("09-tcl85-features");
    /// The 85-builtin command-availability sweep
    /// (`suites/08-stock-84-builtins.list`, `results/08-command-availability.tsv`).
    pub const COMMAND_AVAILABILITY: Self = Self("08-command-availability");
    /// The 120-cell event-context sweep
    /// (`suites/07-event-context.commands`, `results/07-event-context.tsv`).
    pub const EVENT_CONTEXT: Self = Self("07-event-context");
    /// The traffic lab (§8) — priority ordering and runtime behaviour.
    pub const TRAFFIC_LAB: Self = Self("08-traffic-lab");

    /// The probe set's identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// Where an observation came from (the review's evidence-discipline
/// table: five kinds of evidence, kept separate).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceSource {
    /// Executed inside the named BIG-IP execution context on a live
    /// appliance. The only source that can establish an embedded-runtime
    /// fact.
    Appliance,
    /// Executed by a Tcl binary installed on the appliance's host OS.
    /// Provenance only — never embedded-runtime proof (E4 step 2).
    HostBinary(&'static str),
}

/// Provenance for one observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EvidenceProvenance {
    /// How the observation was obtained.
    pub source: EvidenceSource,
    /// The checked-in probe set it came from.
    pub probe_set: ProbeSetId,
    /// The measurements-document section that reports it (`"§4a"`).
    pub section: &'static str,
    /// Whether the run implemented the §E4 probe-and-cleanup contract.
    /// §11 is explicit that only §3 and §4a did; everything else is a
    /// strong but non-conforming transcript, and a consumer weighing
    /// evidence is entitled to know which it is holding.
    pub e4_conforming: bool,
}

/// The value of a Tcl global that a context may not define at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlobalValue {
    /// The global exists with this value.
    Present(&'static str),
    /// The global does not exist. Measured for `tcl_patchLevel` in
    /// `TmshCliScript`, where an unguarded read aborts the probe — as it
    /// did on the run's first attempt (§4a).
    Unset,
}

/// How a context populates `tcl_platform` — the three-way split F5's
/// finding turned out to be (§4/§4a), not the two-way one it was filed as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformShape {
    /// Fabricated BIG-IP values: `os BIG-IP`, `osVersion` the TMOS
    /// release, `tmmVersion`, `machine` = the **hostname**, and no
    /// `nameofexecutable`.
    FabricatedBigIp,
    /// Real host values reported by the interpreter's own build.
    RealHost,
    /// The array exists but has **no elements at all**.
    Empty,
}

/// A typed observation about one execution context on one build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeFact {
    /// What the interpreter reports about its own Tcl release: `info
    /// patchlevel`, and the `tcl_patchLevel` global, which is **not**
    /// always the same question (§4a).
    ReportedPatchlevel {
        /// `info patchlevel`.
        info_patchlevel: &'static str,
        /// The `tcl_patchLevel` global.
        tcl_patch_level_global: GlobalValue,
    },
    /// `llength [info commands]` — 152 / 95 / 95 / 85 across TMM, tmsh,
    /// iApp and the host 8.4 binary (§4a).
    CommandCount(u16),
    /// The shape of `tcl_platform`, and its `wordSize` when it has one.
    TclPlatform {
        /// How the array is populated.
        shape: PlatformShape,
        /// Number of keys.
        keys: u8,
        /// `tcl_platform(wordSize)`, absent when the array is empty.
        word_size: Option<u8>,
    },
    /// Whether `tmsh::version` exists in the context, and what it returns.
    TmshVersion(Option<&'static str>),
    /// One of the sixteen features that cleanly separate stock 8.4 from
    /// stock 8.5, and how this context behaved on it.
    ReleaseDiscriminator {
        /// The probe case id from `suites/09-tcl85-features.tcl`, or the
        /// parity case id for the four measured in `TmmIRule`.
        feature: &'static str,
        /// The measured behaviour.
        behaviour: DiscriminatorBehaviour,
    },
    /// Whether one command exists, and — for iRules — by which of the two
    /// measured mechanisms it does not (§4b).
    CommandSurface {
        /// The command name.
        command: &'static str,
        /// The measured presence class.
        presence: CommandPresence,
    },
    /// When a literal command head is resolved: at rule load (TMM, even
    /// inside `catch`) or at runtime (every other context) — §4a's
    /// load-time resolution finding.
    CommandResolution(ResolutionTime),
    /// The `when` handler priority policy measured in the traffic lab
    /// (§6/§8).
    EventHandlerPriority {
        /// Smallest accepted value.
        min: u16,
        /// Largest accepted value.
        max: u16,
        /// The value an omitted `priority` clause takes.
        default: u16,
        /// Whether a lower number runs first.
        lower_runs_first: bool,
    },
}

/// How a context behaved on an 8.4-vs-8.5 discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscriminatorBehaviour {
    /// Behaved as stock 8.4 — the feature is absent.
    BehavesAs84,
    /// Behaved as stock 8.5 — the feature is present.
    BehavesAs85,
    /// **Appeared** to pass while doing something else entirely. Measured
    /// for `{*}`: the implicit word break turns `{*}$l` into a literal `*`
    /// plus the unexpanded list, so the probe's `catch` succeeds and the
    /// *value is wrong* (§1, §3 row 6). The one divergence in the whole
    /// document with no failure signal at all.
    FalsePass,
}

/// The measured presence class of one command in one context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandPresence {
    /// Present and callable.
    Present,
    /// Absent from the interpreter build: `invalid command name` even
    /// through `eval` at runtime (§4b's first group of 16).
    InterpreterAbsent,
    /// Present in the interpreter, refused by the rule compiler at load:
    /// reachable through `eval`, and `rename` demonstrably works (§4b's
    /// second group of 15).
    CompilerRefused,
}

/// When a context resolves a literal command head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolutionTime {
    /// At rule load, unaffected by `catch` — so "unknown command" is safe
    /// to surface as an error rather than a hint (§4a).
    RuleLoad,
    /// At runtime, like ordinary Tcl.
    Runtime,
}

/// The queryable identity of a fact: everything a lookup keys on except
/// the context and the build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeFactKind {
    /// [`RuntimeFact::ReportedPatchlevel`].
    ReportedPatchlevel,
    /// [`RuntimeFact::CommandCount`].
    CommandCount,
    /// [`RuntimeFact::TclPlatform`].
    TclPlatform,
    /// [`RuntimeFact::TmshVersion`].
    TmshVersion,
    /// [`RuntimeFact::ReleaseDiscriminator`] for one named feature.
    ReleaseDiscriminator(&'static str),
    /// [`RuntimeFact::CommandSurface`] for one named command.
    CommandSurface(&'static str),
    /// [`RuntimeFact::CommandResolution`].
    CommandResolution,
    /// [`RuntimeFact::EventHandlerPriority`].
    EventHandlerPriority,
}

impl RuntimeFact {
    /// The lookup key this fact answers.
    #[must_use]
    pub const fn kind(self) -> RuntimeFactKind {
        match self {
            Self::ReportedPatchlevel { .. } => RuntimeFactKind::ReportedPatchlevel,
            Self::CommandCount(_) => RuntimeFactKind::CommandCount,
            Self::TclPlatform { .. } => RuntimeFactKind::TclPlatform,
            Self::TmshVersion(_) => RuntimeFactKind::TmshVersion,
            Self::ReleaseDiscriminator { feature, .. } => {
                RuntimeFactKind::ReleaseDiscriminator(feature)
            }
            Self::CommandSurface { command, .. } => RuntimeFactKind::CommandSurface(command),
            Self::CommandResolution(_) => RuntimeFactKind::CommandResolution,
            Self::EventHandlerPriority { .. } => RuntimeFactKind::EventHandlerPriority,
        }
    }
}

/// One measured fact about one execution context on one BIG-IP build,
/// with its provenance (F2's required change).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmbeddedRuntimeEvidence {
    /// The execution context the observation was taken in.
    pub context: BigIpExecutionContext,
    /// The appliance build it was taken on.
    pub build: BigIpBuild,
    /// What was observed.
    pub fact: RuntimeFact,
    /// Where the observation came from.
    pub provenance: EvidenceProvenance,
}

impl EmbeddedRuntimeEvidence {
    /// Whether this record may be read as evidence about an **embedded**
    /// BIG-IP runtime.
    ///
    /// False for every [`EvidenceSource::HostBinary`] row. Those rows
    /// exist precisely to falsify the substitution the review objected to
    /// — the host `tclsh8.4` is 8.4.13 while all three F5 contexts embed
    /// 8.4.6 — and must never be consumed as if they described TMM,
    /// `scriptd`, or tmsh.
    #[must_use]
    pub const fn is_embedded_runtime_proof(&self) -> bool {
        matches!(self.provenance.source, EvidenceSource::Appliance)
            && self.context.is_appliance_hosted()
    }
}

/// Why a query could not be answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnknownReason {
    /// The context has never been exercised on any build — the two APL
    /// contexts. No other context's row may stand in for it.
    ContextNeverMeasured,
    /// The context is measured, but this fact was never probed in it.
    FactNeverProbedInContext,
}

/// The answer to an assistance-grade evidence query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceResolution {
    /// Measured on exactly the requested build in exactly the requested
    /// context.
    Measured(&'static EmbeddedRuntimeEvidence),
    /// Not measured on the requested build. This is the nearest known
    /// build **in the same context**, and any surface presenting it must
    /// say so — it is an assistance answer, never a semantic one.
    NearestKnownAssistance {
        /// The nearest-known row.
        evidence: &'static EmbeddedRuntimeEvidence,
        /// The release the caller actually asked about.
        requested_release: &'static str,
    },
    /// No answer. Consumers abstain.
    Unknown(UnknownReason),
}

impl EvidenceResolution {
    /// The record behind a [`Self::Measured`] answer — `None` for both
    /// assistance and unknown, so a semantic consumer cannot reach an
    /// unmeasured row by accident.
    #[must_use]
    pub const fn measured(self) -> Option<&'static EmbeddedRuntimeEvidence> {
        match self {
            Self::Measured(evidence) => Some(evidence),
            Self::NearestKnownAssistance { .. } | Self::Unknown(_) => None,
        }
    }

    /// Whether the answer must be labelled as assistance when shown.
    #[must_use]
    pub const fn is_assistance(self) -> bool {
        matches!(self, Self::NearestKnownAssistance { .. })
    }
}

/// Every evidence row for `context`, in table order.
pub fn evidence_for(
    context: BigIpExecutionContext,
) -> impl Iterator<Item = &'static EmbeddedRuntimeEvidence> {
    EMBEDDED_RUNTIME_EVIDENCE
        .iter()
        .filter(move |row| row.context == context)
}

/// The **semantic** door: the fact measured in exactly this context on
/// exactly this build, or `None`.
///
/// No widening of any kind. A semantic pass that cannot find its row
/// abstains — which is what F2 means by *"an unmeasured BIG-IP release
/// resolves to `Unknown`"*.
#[must_use]
pub fn measured_fact(
    context: BigIpExecutionContext,
    build: BigIpBuild,
    kind: RuntimeFactKind,
) -> Option<&'static EmbeddedRuntimeEvidence> {
    EMBEDDED_RUNTIME_EVIDENCE
        .iter()
        .find(|row| row.context == context && row.build == build && row.fact.kind() == kind)
}

/// The **assistance** door: the measured row when there is one, otherwise
/// an explicitly labelled nearest-known row **from the same context**,
/// otherwise [`EvidenceResolution::Unknown`].
///
/// "Nearest" is measured on the BIG-IP release axis: the greatest measured
/// release not above the requested one, falling back to the smallest
/// measured release above it. It never crosses into another context, and
/// it never invents a row for a context that has none.
#[must_use]
pub fn assistance_fact(
    context: BigIpExecutionContext,
    requested_release: &'static str,
    kind: RuntimeFactKind,
) -> EvidenceResolution {
    let mut candidates = EMBEDDED_RUNTIME_EVIDENCE
        .iter()
        .filter(|row| row.context == context && row.fact.kind() == kind)
        .peekable();
    if candidates.peek().is_none() {
        return EvidenceResolution::Unknown(if context.measurement().is_measured() {
            UnknownReason::FactNeverProbedInContext
        } else {
            UnknownReason::ContextNeverMeasured
        });
    }

    let mut below: Option<&'static EmbeddedRuntimeEvidence> = None;
    let mut above: Option<&'static EmbeddedRuntimeEvidence> = None;
    for row in candidates {
        match compare_versions(row.build.release, requested_release) {
            std::cmp::Ordering::Equal => return EvidenceResolution::Measured(row),
            std::cmp::Ordering::Less => {
                if below.is_none_or(|best| {
                    compare_versions(row.build.release, best.build.release)
                        == std::cmp::Ordering::Greater
                }) {
                    below = Some(row);
                }
            }
            std::cmp::Ordering::Greater => {
                if above.is_none_or(|best| {
                    compare_versions(row.build.release, best.build.release)
                        == std::cmp::Ordering::Less
                }) {
                    above = Some(row);
                }
            }
        }
    }
    match below.or(above) {
        Some(evidence) => EvidenceResolution::NearestKnownAssistance {
            evidence,
            requested_release,
        },
        None => EvidenceResolution::Unknown(UnknownReason::FactNeverProbedInContext),
    }
}

// ---------------------------------------------------------------------
// The seeded table: BIG-IP 21.1.0.1 build 0.0.26, probed 2026-08-26.
// ---------------------------------------------------------------------

const BUILD: BigIpBuild = BigIpBuild::MEASURED_21_1_0_1;

const fn appliance(probe_set: ProbeSetId, section: &'static str, e4: bool) -> EvidenceProvenance {
    EvidenceProvenance {
        source: EvidenceSource::Appliance,
        probe_set,
        section,
        e4_conforming: e4,
    }
}

// §11's own recommendation is the rule for `e4_conforming`: *"Treat §3 and
// §4a as E4-grade evidence and everything else as a strong but
// non-conforming transcript."* The §4b/§4c transcripts were in fact
// produced by `lib/e4-context-probe.sh` and carry cleanup proofs, but the
// document has not promoted them, so neither does this table — an
// evidence layer that grades itself more generously than its own source
// document is exactly the failure mode F8 exists to prevent.
const PARITY: EvidenceProvenance = appliance(ProbeSetId::CONTEXT_PARITY, "§4a", true);
const FEATURES: EvidenceProvenance = appliance(ProbeSetId::RELEASE_FEATURES, "§4", false);
const SURFACE: EvidenceProvenance = appliance(ProbeSetId::PROC_SEMANTICS, "§4b", false);
const LAB: EvidenceProvenance = appliance(ProbeSetId::TRAFFIC_LAB, "§8", false);
const HOST: EvidenceProvenance = EvidenceProvenance {
    source: EvidenceSource::HostBinary("/usr/bin/tclsh8.4"),
    probe_set: ProbeSetId::CONTEXT_PARITY,
    section: "§4a",
    e4_conforming: true,
};

const fn row(
    context: BigIpExecutionContext,
    fact: RuntimeFact,
    provenance: EvidenceProvenance,
) -> EmbeddedRuntimeEvidence {
    EmbeddedRuntimeEvidence {
        context,
        build: BUILD,
        fact,
        provenance,
    }
}

const fn discriminator(
    context: BigIpExecutionContext,
    feature: &'static str,
    behaviour: DiscriminatorBehaviour,
    provenance: EvidenceProvenance,
) -> EmbeddedRuntimeEvidence {
    row(
        context,
        RuntimeFact::ReleaseDiscriminator { feature, behaviour },
        provenance,
    )
}

const fn surface(
    context: BigIpExecutionContext,
    command: &'static str,
    presence: CommandPresence,
    provenance: EvidenceProvenance,
) -> EmbeddedRuntimeEvidence {
    row(
        context,
        RuntimeFact::CommandSurface { command, presence },
        provenance,
    )
}

use BigIpExecutionContext::{HostShellTcl, IAppImplementation, TmmIRule, TmshCliScript};
use CommandPresence::{CompilerRefused, InterpreterAbsent, Present};
use DiscriminatorBehaviour::{BehavesAs84, FalsePass};

/// The sixteen features that cleanly separate stock 8.4 from stock 8.5
/// (§4). Probe case ids are `suites/09-tcl85-features.tcl`'s own.
///
/// All sixteen behave as 8.4 in both F5 contexts they were probed in. The
/// lone apparent pass — `{*}` — is the implicit-word-break artefact, which
/// is why it is [`DiscriminatorBehaviour::FalsePass`] and not
/// [`DiscriminatorBehaviour::BehavesAs85`].
pub const RELEASE_DISCRIMINATORS: [(&str, DiscriminatorBehaviour); 16] = [
    ("expand_op", FalsePass),
    ("dict", BehavesAs84),
    ("lassign", BehavesAs84),
    ("apply", BehavesAs84),
    ("lreverse", BehavesAs84),
    ("lrepeat", BehavesAs84),
    ("string_reverse", BehavesAs84),
    ("pow_operator", BehavesAs84),
    ("in_operator", BehavesAs84),
    ("ni_operator", BehavesAs84),
    ("mathop_ns", BehavesAs84),
    ("chan_cmd", BehavesAs84),
    ("switch_matchvar", BehavesAs84),
    ("string_is_wide", BehavesAs84),
    ("info_frame", BehavesAs84),
    ("namespace_ens", BehavesAs84),
];

/// The four discriminators that were *also* exercised in `TmmIRule`, via
/// the four-context parity list (§4a). The other twelve were probed only
/// in the tmsh and iApp interpreters (§4's own wording), and this module
/// does not claim them for TMM.
const IRULE_MEASURED_DISCRIMINATORS: [(&str, DiscriminatorBehaviour); 4] = [
    ("expand_op", FalsePass),
    ("dict", BehavesAs84),
    ("lassign", BehavesAs84),
    ("apply", BehavesAs84),
];

/// Every seeded evidence row.
///
/// Assembled once from the measured tables above — the per-context facts
/// written out, then the discriminator and command-surface tables expanded
/// over the contexts they were actually probed in. Adding a build means
/// adding rows here; no consumer changes, because every lookup goes
/// through [`measured_fact`] or [`assistance_fact`].
pub static EMBEDDED_RUNTIME_EVIDENCE: std::sync::LazyLock<Vec<EmbeddedRuntimeEvidence>> =
    std::sync::LazyLock::new(|| {
        let mut rows = vec![
            // ---- TmmIRule (§4a) ----
            row(
                TmmIRule,
                RuntimeFact::ReportedPatchlevel {
                    info_patchlevel: "8.4.6",
                    tcl_patch_level_global: GlobalValue::Present("8.4.6"),
                },
                PARITY,
            ),
            row(TmmIRule, RuntimeFact::CommandCount(152), PARITY),
            row(
                TmmIRule,
                RuntimeFact::TclPlatform {
                    shape: PlatformShape::FabricatedBigIp,
                    keys: 7,
                    word_size: Some(8),
                },
                PARITY,
            ),
            row(TmmIRule, RuntimeFact::TmshVersion(None), PARITY),
            row(
                TmmIRule,
                RuntimeFact::CommandResolution(ResolutionTime::RuleLoad),
                PARITY,
            ),
            row(
                TmmIRule,
                RuntimeFact::EventHandlerPriority {
                    min: 0,
                    max: 1000,
                    default: 500,
                    lower_runs_first: true,
                },
                LAB,
            ),
            // ---- TmshCliScript (§4a) ----
            row(
                TmshCliScript,
                RuntimeFact::ReportedPatchlevel {
                    info_patchlevel: "8.4.6",
                    tcl_patch_level_global: GlobalValue::Unset,
                },
                PARITY,
            ),
            row(TmshCliScript, RuntimeFact::CommandCount(95), PARITY),
            row(
                TmshCliScript,
                RuntimeFact::TclPlatform {
                    shape: PlatformShape::Empty,
                    keys: 0,
                    word_size: None,
                },
                PARITY,
            ),
            row(
                TmshCliScript,
                RuntimeFact::TmshVersion(Some("21.1.0.1")),
                PARITY,
            ),
            row(
                TmshCliScript,
                RuntimeFact::CommandResolution(ResolutionTime::Runtime),
                PARITY,
            ),
            surface(TmshCliScript, "exec", Present, PARITY),
            // ---- IAppImplementation (§4/§4a) ----
            row(
                IAppImplementation,
                RuntimeFact::ReportedPatchlevel {
                    info_patchlevel: "8.4.6",
                    tcl_patch_level_global: GlobalValue::Present("8.4.6"),
                },
                PARITY,
            ),
            row(IAppImplementation, RuntimeFact::CommandCount(95), PARITY),
            row(
                IAppImplementation,
                RuntimeFact::TclPlatform {
                    shape: PlatformShape::RealHost,
                    keys: 7,
                    word_size: Some(4),
                },
                PARITY,
            ),
            row(
                IAppImplementation,
                RuntimeFact::TmshVersion(Some("21.1.0.1")),
                PARITY,
            ),
            row(
                IAppImplementation,
                RuntimeFact::CommandResolution(ResolutionTime::Runtime),
                PARITY,
            ),
            surface(IAppImplementation, "exec", Present, PARITY),
            // ---- HostShellTcl — provenance only (§4a, E4 step 2) ----
            row(
                HostShellTcl,
                RuntimeFact::ReportedPatchlevel {
                    info_patchlevel: "8.4.13",
                    tcl_patch_level_global: GlobalValue::Present("8.4.13"),
                },
                HOST,
            ),
            row(HostShellTcl, RuntimeFact::CommandCount(85), HOST),
            row(
                HostShellTcl,
                RuntimeFact::TclPlatform {
                    shape: PlatformShape::RealHost,
                    keys: 8,
                    word_size: Some(8),
                },
                HOST,
            ),
            row(HostShellTcl, RuntimeFact::TmshVersion(None), HOST),
            row(
                HostShellTcl,
                RuntimeFact::CommandResolution(ResolutionTime::Runtime),
                HOST,
            ),
        ];

        // The 16/16 discriminators, in the two contexts §4 probed them in.
        for context in [TmshCliScript, IAppImplementation] {
            rows.extend(
                RELEASE_DISCRIMINATORS.iter().map(|&(feature, behaviour)| {
                    discriminator(context, feature, behaviour, FEATURES)
                }),
            );
        }
        // …and the four the parity list also exercised in TMM.
        rows.extend(
            IRULE_MEASURED_DISCRIMINATORS
                .iter()
                .map(|&(feature, behaviour)| discriminator(TmmIRule, feature, behaviour, PARITY)),
        );

        // §4b's two mechanisms, as per-command TMM surface facts.
        rows.extend(
            IRULES_INTERPRETER_ABSENT
                .iter()
                .map(|&command| surface(TmmIRule, command, InterpreterAbsent, SURFACE)),
        );
        rows.extend(
            IRULES_COMPILER_REFUSED
                .iter()
                .map(|&command| surface(TmmIRule, command, CompilerRefused, SURFACE)),
        );

        rows
    });

#[cfg(test)]
mod tests {
    use super::*;

    fn find(
        context: BigIpExecutionContext,
        kind: RuntimeFactKind,
    ) -> &'static EmbeddedRuntimeEvidence {
        measured_fact(context, BigIpBuild::MEASURED_21_1_0_1, kind)
            .unwrap_or_else(|| panic!("{context} has no {kind:?} row"))
    }

    /// Every row is keyed uniquely by `(context, build, fact kind)` — a
    /// duplicate would make lookups order-dependent and hide a
    /// contradiction between two transcripts.
    #[test]
    fn evidence_rows_are_uniquely_keyed() {
        let mut seen = std::collections::HashSet::new();
        for evidence in EMBEDDED_RUNTIME_EVIDENCE.iter() {
            assert!(
                seen.insert((evidence.context, evidence.build, evidence.fact.kind())),
                "duplicate row for {} / {:?}",
                evidence.context,
                evidence.fact.kind()
            );
            assert!(
                MEASURED_BUILDS.contains(&evidence.build),
                "{} cites an unlisted build",
                evidence.context
            );
            assert!(
                !evidence.provenance.section.is_empty(),
                "{} has no section citation",
                evidence.context
            );
        }
    }

    /// F2's headline correction: all three F5 contexts embed **8.4.6**,
    /// while the host binary on the same appliance is **8.4.13**. Reading
    /// the version off the host would have been wrong for every F5 row.
    #[test]
    fn the_host_binary_is_not_the_embedded_runtime() {
        for context in [TmmIRule, TmshCliScript, IAppImplementation] {
            let RuntimeFact::ReportedPatchlevel {
                info_patchlevel, ..
            } = find(context, RuntimeFactKind::ReportedPatchlevel).fact
            else {
                panic!("{context}: wrong fact variant");
            };
            assert_eq!(info_patchlevel, "8.4.6", "{context}");
        }
        let host = find(HostShellTcl, RuntimeFactKind::ReportedPatchlevel);
        let RuntimeFact::ReportedPatchlevel {
            info_patchlevel, ..
        } = host.fact
        else {
            panic!("wrong fact variant");
        };
        assert_eq!(info_patchlevel, "8.4.13");
        assert!(!host.is_embedded_runtime_proof());
        assert!(matches!(
            host.provenance.source,
            EvidenceSource::HostBinary(_)
        ));

        // Everything appliance-sourced in a BIG-IP context is proof.
        for evidence in EMBEDDED_RUNTIME_EVIDENCE.iter() {
            assert_eq!(
                evidence.is_embedded_runtime_proof(),
                evidence.context.is_appliance_hosted(),
                "{} / {:?}",
                evidence.context,
                evidence.fact.kind()
            );
        }
    }

    /// `tcl_patchLevel` is unset in a `cli script` and its `tcl_platform`
    /// is empty — the two facts that aborted the probe's first run. The
    /// platform split is three-way, not two-way (F5's finding, refined).
    #[test]
    fn the_platform_split_is_three_way() {
        let shapes: Vec<_> = [TmmIRule, TmshCliScript, IAppImplementation]
            .into_iter()
            .map(
                |context| match find(context, RuntimeFactKind::TclPlatform).fact {
                    RuntimeFact::TclPlatform {
                        shape,
                        keys,
                        word_size,
                    } => (shape, keys, word_size),
                    other => panic!("{context}: {other:?}"),
                },
            )
            .collect();
        assert_eq!(
            shapes,
            vec![
                (PlatformShape::FabricatedBigIp, 7, Some(8)),
                (PlatformShape::Empty, 0, None),
                (PlatformShape::RealHost, 7, Some(4)),
            ]
        );

        let RuntimeFact::ReportedPatchlevel {
            tcl_patch_level_global,
            ..
        } = find(TmshCliScript, RuntimeFactKind::ReportedPatchlevel).fact
        else {
            panic!("wrong fact variant");
        };
        assert_eq!(tcl_patch_level_global, GlobalValue::Unset);
    }

    /// Sixteen discriminators in the two contexts §4 probed, four in TMM,
    /// and `{*}` is a false pass everywhere rather than an 8.5 feature.
    #[test]
    fn the_discriminators_are_claimed_only_where_they_were_probed() {
        for context in [TmshCliScript, IAppImplementation] {
            let measured: Vec<_> = evidence_for(context)
                .filter(|row| matches!(row.fact, RuntimeFact::ReleaseDiscriminator { .. }))
                .collect();
            assert_eq!(measured.len(), 16, "{context}");
            for row in measured {
                let RuntimeFact::ReleaseDiscriminator { feature, behaviour } = row.fact else {
                    unreachable!()
                };
                let expected = if feature == "expand_op" {
                    FalsePass
                } else {
                    BehavesAs84
                };
                assert_eq!(behaviour, expected, "{context}/{feature}");
                assert_ne!(behaviour, DiscriminatorBehaviour::BehavesAs85);
            }
        }
        let irule: Vec<_> = evidence_for(TmmIRule)
            .filter(|row| matches!(row.fact, RuntimeFact::ReleaseDiscriminator { .. }))
            .collect();
        assert_eq!(irule.len(), 4, "only the parity list's four in TMM");
        assert_eq!(
            measured_fact(
                TmmIRule,
                BigIpBuild::MEASURED_21_1_0_1,
                RuntimeFactKind::ReleaseDiscriminator("chan_cmd"),
            ),
            None,
            "never probed in TMM — must not be claimed"
        );
    }

    /// The unmeasured-build rule (F2): the semantic door answers `None`
    /// and the assistance door answers a **labelled** nearest-known row.
    #[test]
    fn an_unmeasured_build_never_silently_inherits() {
        let kind = RuntimeFactKind::ReportedPatchlevel;
        let other_build = BigIpBuild {
            release: "17.1.1",
            build: "0.0.4",
        };
        assert_eq!(measured_fact(TmmIRule, other_build, kind), None);

        let resolution = assistance_fact(TmmIRule, "17.1.1", kind);
        let EvidenceResolution::NearestKnownAssistance {
            evidence,
            requested_release,
        } = resolution
        else {
            panic!("{resolution:?}");
        };
        assert_eq!(requested_release, "17.1.1");
        assert_eq!(evidence.build, BigIpBuild::MEASURED_21_1_0_1);
        assert_eq!(evidence.context, TmmIRule, "assistance stays in-context");
        assert!(resolution.is_assistance());
        assert_eq!(resolution.measured(), None, "assistance is not semantics");

        // The measured build answers through both doors.
        let exact = assistance_fact(TmmIRule, "21.1.0.1", kind);
        assert!(!exact.is_assistance());
        assert!(exact.measured().is_some());
        assert!(measured_fact(TmmIRule, BigIpBuild::MEASURED_21_1_0_1, kind).is_some());
    }

    /// The two APL contexts stay `Unknown` through **both** doors, and the
    /// reason distinguishes "never measured" from "not probed here".
    #[test]
    fn the_apl_contexts_never_inherit_the_implementation_row() {
        for context in [
            BigIpExecutionContext::IAppPresentationApl,
            BigIpExecutionContext::IAppPresentationTclCallback,
        ] {
            assert_eq!(evidence_for(context).count(), 0, "{context}");
            for kind in [
                RuntimeFactKind::ReportedPatchlevel,
                RuntimeFactKind::TclPlatform,
                RuntimeFactKind::CommandSurface("exec"),
            ] {
                assert_eq!(
                    measured_fact(context, BigIpBuild::MEASURED_21_1_0_1, kind),
                    None,
                    "{context}"
                );
                assert_eq!(
                    assistance_fact(context, "21.1.0.1", kind),
                    EvidenceResolution::Unknown(UnknownReason::ContextNeverMeasured),
                    "{context}"
                );
            }
        }

        // A measured context that simply lacks a probe reports the other
        // reason — the two are not interchangeable.
        assert_eq!(
            assistance_fact(
                TmshCliScript,
                "21.1.0.1",
                RuntimeFactKind::EventHandlerPriority
            ),
            EvidenceResolution::Unknown(UnknownReason::FactNeverProbedInContext)
        );
    }

    /// F4's concrete case: `exec` is absent in TMM and works in the other
    /// two F5 contexts, so a command fact must never be promoted across
    /// contexts.
    #[test]
    fn command_surface_facts_do_not_cross_contexts() {
        let kind = RuntimeFactKind::CommandSurface("exec");
        let build = BigIpBuild::MEASURED_21_1_0_1;
        let presence = |context| match measured_fact(context, build, kind)
            .unwrap_or_else(|| panic!("{context}: no exec row"))
            .fact
        {
            RuntimeFact::CommandSurface { presence, .. } => presence,
            other => panic!("{other:?}"),
        };
        assert_eq!(presence(TmmIRule), InterpreterAbsent);
        assert_eq!(presence(TmshCliScript), Present);
        assert_eq!(presence(IAppImplementation), Present);
        assert_eq!(measured_fact(HostShellTcl, build, kind), None);

        // The §4b split is complete for TMM: 16 + 15 command rows.
        let absent = evidence_for(TmmIRule)
            .filter(|row| {
                matches!(
                    row.fact,
                    RuntimeFact::CommandSurface {
                        presence: InterpreterAbsent,
                        ..
                    }
                )
            })
            .count();
        let refused = evidence_for(TmmIRule)
            .filter(|row| {
                matches!(
                    row.fact,
                    RuntimeFact::CommandSurface {
                        presence: CompilerRefused,
                        ..
                    }
                )
            })
            .count();
        assert_eq!((absent, refused), (16, 15));
    }

    /// §11's E4-conformance delta, as data: only the §4a rows are graded
    /// E4-conforming, so a consumer weighing evidence can tell a ratified
    /// row from a strong-but-non-conforming one. Re-running the other
    /// suites under the E4 contract is what flips these flags.
    #[test]
    fn the_e4_delta_is_recorded_per_row() {
        let mut conforming = 0;
        let mut non_conforming = 0;
        for evidence in EMBEDDED_RUNTIME_EVIDENCE.iter() {
            if evidence.provenance.e4_conforming {
                assert_eq!(
                    evidence.provenance.section, "§4a",
                    "only §4a is E4-grade (§11's recommendation)"
                );
                conforming += 1;
            } else {
                assert_ne!(evidence.provenance.section, "§4a");
                non_conforming += 1;
            }
        }
        assert!(conforming > 0 && non_conforming > 0);
        // The whole §4b command surface is non-conforming today.
        assert!(
            evidence_for(TmmIRule)
                .filter(|row| matches!(row.fact, RuntimeFact::CommandSurface { .. }))
                .all(|row| !row.provenance.e4_conforming)
        );
    }

    /// The traffic lab's priority policy is evidence, and the shipping
    /// registry constant must agree with it exactly.
    #[test]
    fn the_priority_policy_matches_the_shipping_constant() {
        let RuntimeFact::EventHandlerPriority {
            min,
            max,
            default,
            lower_runs_first,
        } = find(TmmIRule, RuntimeFactKind::EventHandlerPriority).fact
        else {
            panic!("wrong fact variant");
        };
        let shipping = crate::events::BIGIP_EVENT_HANDLER_PRIORITY;
        assert_eq!(min, shipping.min_priority);
        assert_eq!(max, shipping.max_priority);
        assert_eq!(default, shipping.default_priority);
        assert_eq!(lower_runs_first, shipping.lower_runs_first);
    }
}
