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

//! The **F5 evidence layer**: which BIG-IP build a fact was measured on,
//! in which execution context, with what provenance — and the hermetic
//! conformance corpus that keeps the model honest about it.
//!
//! The rulings this implements are in
//! `docs/design/dialect-and-package-registry-redesign.md` §0.2 (the F5
//! evidence-review disposition table), the review itself
//! (`dialect-and-package-registry-redesign-bigip-evidence-review.md`,
//! findings F1–F8), R11 in
//! `dialect-and-package-registry-centralisation.md` §4, and the live
//! transcript in `docs/design/bigip-irule-parser-measurements.md`. Section
//! references in this module (`§4a`, `§4b`, `§8`, …) are that measurements
//! document unless said otherwise.
//!
//! Four layers, and the direction of dependence matters:
//!
//! - [`execution_context`] — [`BigIpExecutionContext`], the F1 key. Six
//!   contexts, four measured, two honestly [`ContextMeasurement::Unmeasured`].
//! - [`evidence`] — [`EmbeddedRuntimeEvidence`], the F2 truth records:
//!   `(context, BIG-IP build, fact, provenance)`. An unmeasured build
//!   resolves to [`EvidenceResolution::Unknown`] or to an explicitly
//!   labelled [`EvidenceResolution::NearestKnownAssistance`], never to a
//!   silent guess — and assistance never crosses a context boundary.
//! - [`corpus`] — the R11 conformance corpus: hermetic vectors derived
//!   from the checked-in transcripts (`scripts/dev/bigip-probes/`), each
//!   row citing its measurements section, each tested against the model so
//!   that a model change contradicting measured behaviour fails a test
//!   rather than shipping.
//! - [`tmsh_syntax`] and [`iapp_metadata`] — F6's third version axis with
//!   its `tmsh::modify cli version active` transition, and F7's
//!   action-local overlays (`requires-bigip-version-min`/`-max`,
//!   `role-acl`, `run-as`).
//!
//! **What this layer is not.** It does not migrate any F5 registry row:
//! the §0.2 migration hold stands until the review's acceptance matrix has
//! its coverage (one 17.x build, one older build, the restricted-role tmsh
//! column, and the two APL contexts). What it does is make the hold
//! *checkable*: every claim now carries the build and context it was
//! measured on, and every place the shipping model disagrees with the
//! appliance is a recorded, asserted corpus row instead of tribal
//! knowledge.

pub mod corpus;
pub mod evidence;
pub mod execution_context;
pub mod iapp_metadata;
pub mod tmsh_syntax;

pub use corpus::{
    COMMAND_CLASS_VECTORS, CONTEXT_ENVIRONMENT_VECTORS, CaseOutcome, CommandClassVector,
    ContextEnvironmentVector, DivergenceReason, EVENT_CONTEXT_VECTORS, EventCell,
    EventContextVector, GrammarAxis, ModelExpectation, ModelProbe, PARSER_PARITY_VECTORS,
    PRIORITY_VECTORS, ParserParityVector, PriorityCase, PriorityVector,
    RELEASE_DISCRIMINATOR_VECTORS, ReleaseDiscriminatorVector,
};
pub use evidence::{
    BigIpBuild, CommandPresence, DiscriminatorBehaviour, EMBEDDED_RUNTIME_EVIDENCE,
    EmbeddedRuntimeEvidence, EvidenceProvenance, EvidenceResolution, EvidenceSource, GlobalValue,
    MEASURED_BUILDS, PlatformShape, ProbeSetId, RELEASE_DISCRIMINATORS, ResolutionTime,
    RuntimeFact, RuntimeFactKind, UnknownReason, assistance_fact, evidence_for, measured_fact,
};
pub use execution_context::{BigIpExecutionContext, ContextMeasurement};
pub use iapp_metadata::{
    IAppActionOverlay, IAppMetadataError, RunAsPrincipal, parse_iapp_action_metadata,
};
pub use tmsh_syntax::{
    TMSH_SYNTAX_VERSIONED_SINCE, TmshSyntaxSelection, TmshSyntaxState, TmshSyntaxTransition,
    tmsh_syntax_transition_for,
};
