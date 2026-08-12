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

//! Types for the var-escape analysis.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use bitflags::bitflags;
use tcl_lexer::Span;

/// Why the analysis fell back to the dynamic-barrier path.
///
/// Each kind names a distinct *reason* so
/// codegen, the LSP, and the compiler explorer can narrow / explain
/// rather than treat every pessimistic-frame proc as opaque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarrierKind {
    /// Frame-inspecting `info` subcommand (`info level`,
    /// `info frame`, `info vars`, `info locals`, `info coroutine`,
    /// `info errorstack`), or an unknown / dynamic `info`
    /// subcommand.
    Info,
    /// `eval` / `source` / `subst` with a body the analysis can't
    /// statically scan (dynamic body, parse failure, lowering
    /// failure).
    Eval,
    /// `upvar` with a dynamic level or unbounded source, or a
    /// non-`#0` `uplevel` whose body could touch caller-frame
    /// names.
    Upvar,
    /// Assignment / increment / unset whose first arg is a
    /// dynamic name the analysis can't resolve to a literal
    /// target (`set $varname value` etc.) — the fallback escapes
    /// every known proc-local.
    DynName,
    /// `{*}`-word expansion in an unknown command call, where
    /// argument-index analysis can't tell where the name landed.
    Expand,
    /// Any other barrier shape we don't yet classify (catch-all
    /// so the field stays exhaustive).
    Unknown,
}

/// A single barrier trigger recorded by the analysis.
///
/// `kind` identifies the category (see [`BarrierKind`]); `detail`
/// is a short human-readable explanation surfaced verbatim in
/// compiler-explorer hovers and LSP hints (e.g. ``"info level"``,
/// ``"eval $body (dynamic)"``, ``"set $varname value"``).  `range`
/// is the source span of the triggering construct when the
/// analysis pass had it available — `None` when the barrier was
/// synthesised from an inferred / nested IR shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Barrier {
    /// Barrier category.
    pub kind: BarrierKind,
    /// Human-readable explanation.
    pub detail: String,
    /// Source span of the triggering construct, if known.
    pub range: Option<Span>,
}

impl Barrier {
    /// Build a barrier of *kind* with empty detail and no range.
    #[must_use]
    pub fn new(kind: BarrierKind) -> Self {
        Self {
            kind,
            detail: String::new(),
            range: None,
        }
    }

    /// Build a barrier of *kind* carrying a human-readable detail.
    #[must_use]
    pub fn with_detail(kind: BarrierKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
            range: None,
        }
    }
}

/// Why a specific variable was tagged `Frame`.
///
/// Per-name companion to [`BarrierKind`].  A barrier is a
/// proc-level fact ("this proc went pessimistic"); an escape
/// reason is a per-variable fact ("this var is `Frame` because
/// …") — they overlap (a barrier *implies* every-known-var is
/// `Frame`) but the finer split lets the LSP / compiler explorer
/// answer "why is `x` `Frame`?" precisely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeReasonKind {
    /// Proc went pessimistic; the var is `Frame` by virtue of
    /// being in the proc.  Pair with the proc's [`Barrier`] for
    /// detail.
    Barrier,
    /// `[info exists <name>]` references this var by literal
    /// name.  The interpreter resolves the lookup against the
    /// runtime frame, so the var must live there.
    InfoExists,
    /// An `info` subcommand *writes* this var by literal name in the
    /// current frame — `info default procname arg varname` stores the
    /// argument's default into `varname` (a registry-declared
    /// [`tcl_registry::ArgRole::VarWrite`]).
    InfoVarWrite,
    /// This name is the *source* of an `upvar` declaration in
    /// this proc.  Caller-side aliasing requires the name to be
    /// in the runtime frame.
    UpvarSource,
    /// Interprocedural: a callee upvars this name from our
    /// frame, so we have to spill it.
    CalleeUpvar,
    /// An `eval` / `source` / `subst` body references this name
    /// (`$<name>` or `set <name> ...`).
    EvalReference,
    /// A dynamic-name command (`set $v ...`) resolved to this
    /// literal target via in-body literal analysis.
    DynNameResolved,
    /// Fallback path spilled every known proc-local because no
    /// narrower escape set could be proved.
    EscapeAllKnown,
}

/// A single per-variable escape reason recorded by the analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EscapeReason {
    /// Reason category.
    pub kind: EscapeReasonKind,
    /// Human-readable explanation.
    pub detail: String,
    /// Source span of the triggering construct, if known.
    pub range: Option<Span>,
}

impl EscapeReason {
    /// Build a reason of *kind* with empty detail and no range.
    #[must_use]
    pub fn new(kind: EscapeReasonKind) -> Self {
        Self {
            kind,
            detail: String::new(),
            range: None,
        }
    }

    /// Build a reason of *kind* carrying a human-readable detail.
    #[must_use]
    pub fn with_detail(kind: EscapeReasonKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
            range: None,
        }
    }
}

/// Where a Tcl variable must live at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeTag {
    /// Only accessed through statically resolved positions; the
    /// WASM local slot is the single source of truth.
    Local,
    /// Must live in the runtime frame so the interpreter (or an
    /// `upvar` alias) can read and write it by name.
    Frame,
}

bitflags! {
    /// Pessimism / fallback flag set shared by the var-escape
    /// analysis structs ([`ProcEscapeSummary`],
    /// [`super::state::EscapeState`],
    /// [`super::cfg_propagation::state::CfgEscapeResult`],
    /// [`super::cfg_propagation::state::CfgState`]).
    ///
    /// Each flag is monotonic (set once, never cleared) and
    /// orthogonal to the others. Stored as a single byte so the
    /// analysis state stays small.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
    pub struct EscapeFlags: u8 {
        /// Whole-proc pessimism marker: `eval $body`, `uplevel 1`,
        /// `info level`, or any other construct whose name-reference
        /// set cannot be bounded. Forces every variable to `Frame`.
        const DYNAMIC_BARRIER        = 0b0001;
        /// The set of caller-frame names this proc reaches via
        /// `upvar` cannot be statically enumerated. Callers must
        /// spill every local.
        const UNBOUNDED_UPVAR_SOURCE = 0b0010;
        /// Codegen needs the eval fallback for this proc — a
        /// definite interpreter dispatch (barrier-shaped).
        const HAS_FALLBACK           = 0b0100;
        /// A non-frameless `Statement::Call` with a statically resolvable
        /// command word was seen. The interprocedural pass decides
        /// whether this reaches the eval fallback.
        const HAS_CALL_FALLBACK      = 0b1000;
    }
}

impl EscapeFlags {
    /// True if the [`Self::DYNAMIC_BARRIER`] flag is set.
    #[must_use]
    pub fn dynamic_barrier(self) -> bool {
        self.contains(Self::DYNAMIC_BARRIER)
    }

    /// True if the [`Self::UNBOUNDED_UPVAR_SOURCE`] flag is set.
    #[must_use]
    pub fn unbounded_upvar_source(self) -> bool {
        self.contains(Self::UNBOUNDED_UPVAR_SOURCE)
    }

    /// True if the [`Self::HAS_FALLBACK`] flag is set.
    #[must_use]
    pub fn has_fallback(self) -> bool {
        self.contains(Self::HAS_FALLBACK)
    }

    /// True if the [`Self::HAS_CALL_FALLBACK`] flag is set.
    #[must_use]
    pub fn has_call_fallback(self) -> bool {
        self.contains(Self::HAS_CALL_FALLBACK)
    }
}

/// Join operator on the lattice: `Frame` dominates `Local`.
#[must_use]
pub fn join(a: EscapeTag, b: EscapeTag) -> EscapeTag {
    if a == EscapeTag::Frame || b == EscapeTag::Frame {
        EscapeTag::Frame
    } else {
        EscapeTag::Local
    }
}

/// Per-procedure escape classification.
///
/// `tags` maps variable name to its escape tag. Names not present
/// default to [`EscapeTag::Local`].
///
/// `flags` carries the four orthogonal pessimism markers
/// ([`EscapeFlags::DYNAMIC_BARRIER`],
/// [`EscapeFlags::UNBOUNDED_UPVAR_SOURCE`],
/// [`EscapeFlags::HAS_FALLBACK`],
/// [`EscapeFlags::HAS_CALL_FALLBACK`]).
///
/// `frame_needed` is a convenience flag for codegen: true if the
/// proc needs a runtime frame at all.
///
/// `upvar_source_names` is the set of literal variable names
/// this proc (or any of its transitive callees once the
/// interprocedural pass has run) names as the *source* of a
/// caller-frame `upvar`. A caller must treat any of its local
/// vars whose names appear here as `Frame`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcEscapeSummary {
    /// Per-name escape tag.
    pub tags: HashMap<String, EscapeTag>,
    /// Pessimism / fallback flag set.
    pub flags: EscapeFlags,
    /// Convenience flag for codegen.
    pub frame_needed: bool,
    /// Names this proc treats as upvar sources.
    pub upvar_source_names: BTreeSet<String>,
    /// Statically resolvable callees.
    pub direct_callees: BTreeSet<String>,
    /// Per-SSA-version escape tags, populated by the flow-sensitive
    /// CFG+SSA propagation. Empty when the analysis was driven
    /// from an IR-only source. Keyed by `(name, ssa_version)`.
    pub ssa_tags: HashMap<(String, u32), EscapeTag>,
    /// Slot indices for proc-locals that pass the slot-resolution
    /// eligibility check.  Empty when the proc isn't eligible at
    /// all (dynamic eval body, `has_fallback`, etc.) or when no
    /// local survives the per-name checks.  Populated by
    /// [`super::slot_resolution::populate_local_slots`].
    pub local_slots: BTreeMap<String, u32>,
    /// Recorded barrier triggers.  Each barrier carries its kind +
    /// detail + source range so callers (codegen, compiler
    /// explorer, LSP) can narrow / explain rather than treat every
    /// pessimistic-frame proc as opaque.  Always consistent with
    /// `dynamic_barrier`: the latter is `true` iff `barriers` is
    /// non-empty *or* the dynamic-barrier flag was set explicitly.
    /// Empty for procs the analysis proved frame-elidable.
    ///
    /// The walker populates this from each frame-crossing barrier it
    /// encounters (`state.rs` `record_barrier`); `reasons_for` also
    /// synthesises a `Barrier` reason from `dynamic_barrier()` so a
    /// summary built without a captured barrier list still surfaces a
    /// structured reason to the LSP / explorer.
    pub barriers: Vec<Barrier>,
    /// Per-variable escape reasons.  `tag_reasons[name]` lists
    /// every trigger that contributed to `tags[name] == Frame`.
    /// An empty `Vec` (or missing key) means the analysis didn't
    /// record a specific reason — typically because the var
    /// stayed `Local` or because a barrier-induced escape is
    /// implied by `barriers` rather than explicit per-name
    /// (`Barrier` reasons are usually synthesised on demand by
    /// [`Self::reasons_for`]).
    pub tag_reasons: HashMap<String, Vec<EscapeReason>>,
    /// **Pure-leaf flag.** `true` when this proc is provably
    /// pure-leaf: no escaping var (no `Frame` tag, no dynamic barrier),
    /// no eval / call fallback, no `upvar` source out, and — after the
    /// interprocedural fixpoint — every direct callee is itself pure-leaf
    /// (or a known frameless runtime builtin). The inliner reads this as
    /// the soundness predicate for splicing a body into a caller.
    ///
    /// The walker sets the *intraprocedural* base value (the per-proc
    /// clause); [`solve_interprocedural_escape`](crate::var_escape::solve_interprocedural_escape)
    /// can only **downgrade** it (a proc whose callee is opaque is no
    /// longer pure-leaf). Default `false` — anything unclassified stays
    /// opaque.
    pub pure_leaf: bool,
}

impl ProcEscapeSummary {
    /// Whether the proc body can be physically relocated into a caller's
    /// IR without changing observable behaviour (the inliner's gate).
    /// Identical to [`Self::pure_leaf`] today; split out so a future
    /// relaxation can separate the body-frame-observation clause from the
    /// transitive callee clause.
    #[must_use]
    pub fn safe_to_inline(&self) -> bool {
        self.pure_leaf
    }

    /// Whether dead-store elimination on this proc's locals is sound — a
    /// relaxation of [`Self::pure_leaf`] that drops the transitive
    /// "callees are pure-leaf" clause (DCE only cares whether an external
    /// observer can read our locals, not what callees do).
    #[must_use]
    pub fn safe_to_dce(&self) -> bool {
        !self.frame_needed
            && !self.has_fallback()
            && !self.has_call_fallback()
            && self.upvar_source_names.is_empty()
            && !self.unbounded_upvar_source()
    }

    /// Whether codegen may omit the per-call frame push/pop for this proc
    /// — the proc's frame is never observed externally. A relaxation of
    /// [`Self::pure_leaf`] that only requires `!frame_needed`.
    #[must_use]
    pub fn safe_for_frame_elision(&self) -> bool {
        !self.frame_needed
    }

    /// True if the whole-proc dynamic-barrier marker is set.
    ///
    /// Returns `true` when either the [`EscapeFlags::DYNAMIC_BARRIER`]
    /// flag is set explicitly *or* the [`Self::barriers`] vector is
    /// non-empty.  Treating populated `barriers` as pessimistic
    /// enforces the documented invariant in code rather than
    /// relying on every future population path to remember to set
    /// the flag alongside pushing a barrier.
    #[must_use]
    pub fn dynamic_barrier(&self) -> bool {
        self.flags.dynamic_barrier() || !self.barriers.is_empty()
    }

    /// True if the upvar-source set can't be statically enumerated.
    #[must_use]
    pub fn unbounded_upvar_source(&self) -> bool {
        self.flags.unbounded_upvar_source()
    }

    /// True if codegen needs the eval fallback for this proc.
    #[must_use]
    pub fn has_fallback(&self) -> bool {
        self.flags.has_fallback()
    }

    /// True if the intraprocedural pass saw a non-frameless
    /// `Statement::Call` with a statically resolvable command word.
    /// Whether that reaches the eval fallback depends on whether the
    /// callee is a compiled proc — only the interprocedural pass
    /// can tell.
    #[must_use]
    pub fn has_call_fallback(&self) -> bool {
        self.flags.has_call_fallback()
    }

    /// Return the tag for *name* (defaults to `Local`).
    #[must_use]
    pub fn tag(&self, name: &str) -> EscapeTag {
        if self.dynamic_barrier() {
            return EscapeTag::Frame;
        }
        self.tags.get(name).copied().unwrap_or(EscapeTag::Local)
    }

    /// Shorthand: does *name* need to live in the runtime frame?
    #[must_use]
    pub fn is_frame(&self, name: &str) -> bool {
        self.tag(name) == EscapeTag::Frame
    }

    /// Single source of truth for "this proc needs a runtime
    /// frame".  Combines every analysis-known reason a proc body
    /// can't run with the frame elided:
    ///
    /// * `frame_needed` — at least one var is `Frame` (escapes to
    ///   a callee's `upvar` source, or the dynamic barrier
    ///   flagged everything `Frame`).
    /// * `has_fallback()` — a statically resolvable command falls
    ///   through to the eval fallback, which resolves names
    ///   against the runtime frame.
    ///
    /// Codegen / LSP / compiler-explorer consumers should call
    /// this property instead of recomposing the same condition at
    /// each call site.  The property does NOT include codegen-time
    /// scans (e.g. `info level` references the analysis missed) —
    /// those are layered on top of this gate by the emitter's
    /// top-level `wants_frame` resolver until the analysis itself
    /// catches every hazard.
    #[must_use]
    pub fn wants_frame(&self) -> bool {
        self.frame_needed || self.has_fallback()
    }

    /// Return the per-variable escape reasons for *name*.
    ///
    /// When the proc went pessimistic (`dynamic_barrier()` is
    /// `true`) and no specific per-name reason was recorded,
    /// synthesise one [`EscapeReason`] with kind
    /// [`EscapeReasonKind::Barrier`] per proc-level [`Barrier`]
    /// so callers can answer "why is `x` `Frame`?" with structured
    /// detail instead of "because everything is".  When the var
    /// is `Local` returns an empty `Vec` — the var is not in the
    /// frame; no reason to surface.
    ///
    /// When `tag_reasons` holds explicit per-name reasons they are
    /// returned directly.  Otherwise this method returns either the
    /// empty `Vec` (var is `Local`) or a single placeholder
    /// `Barrier` reason (var is `Frame` due to `dynamic_barrier`).
    #[must_use]
    pub fn reasons_for(&self, name: &str) -> Vec<EscapeReason> {
        if !self.is_frame(name) {
            return Vec::new();
        }
        if let Some(explicit) = self.tag_reasons.get(name)
            && !explicit.is_empty()
        {
            return explicit.clone();
        }
        if self.dynamic_barrier() {
            // Synthesise from proc-level barriers when present so
            // the LSP / compiler explorer has structured detail to
            // surface even when nothing specific was recorded for
            // this var.
            if !self.barriers.is_empty() {
                return self
                    .barriers
                    .iter()
                    .map(|b| EscapeReason {
                        kind: EscapeReasonKind::Barrier,
                        detail: b.detail.clone(),
                        range: b.range,
                    })
                    .collect();
            }
            // No specific barrier recorded — return a single
            // placeholder so consumers still get a structured
            // reason rather than an empty `Vec`.
            return vec![EscapeReason::new(EscapeReasonKind::Barrier)];
        }
        Vec::new()
    }

    /// Return a new summary with *`extra_escaped`* spilled to
    /// `Frame`. Used by the interprocedural pass to fold
    /// callee-induced escapes into a caller's summary without
    /// mutating the originally computed structure.
    #[must_use]
    pub fn with_escapes<I: IntoIterator<Item = String>>(
        &self,
        extra_escaped: I,
        pessimistic: bool,
    ) -> Self {
        let mut new_tags = self.tags.clone();
        let mut any_extra = false;
        for name in extra_escaped {
            new_tags.insert(name, EscapeTag::Frame);
            any_extra = true;
        }
        let new_pessimistic = self.dynamic_barrier() || pessimistic;
        let new_frame_needed = new_pessimistic || new_tags.values().any(|t| *t == EscapeTag::Frame);
        let mut new_flags = self.flags;
        new_flags.set(EscapeFlags::DYNAMIC_BARRIER, new_pessimistic);
        // A callee-induced escape or a pessimistic downgrade clears
        // pure_leaf.
        let new_pure_leaf = self.pure_leaf && !any_extra && !new_pessimistic;
        Self {
            tags: new_tags,
            flags: new_flags,
            frame_needed: new_frame_needed,
            upvar_source_names: self.upvar_source_names.clone(),
            direct_callees: self.direct_callees.clone(),
            ssa_tags: self.ssa_tags.clone(),
            local_slots: self.local_slots.clone(),
            barriers: self.barriers.clone(),
            tag_reasons: self.tag_reasons.clone(),
            pure_leaf: new_pure_leaf,
        }
    }

    /// Return a new summary with *slots* installed as the
    /// `local_slots` field.
    #[must_use]
    pub fn with_local_slots(&self, slots: BTreeMap<String, u32>) -> Self {
        Self {
            tags: self.tags.clone(),
            flags: self.flags,
            frame_needed: self.frame_needed,
            upvar_source_names: self.upvar_source_names.clone(),
            direct_callees: self.direct_callees.clone(),
            ssa_tags: self.ssa_tags.clone(),
            local_slots: slots,
            barriers: self.barriers.clone(),
            tag_reasons: self.tag_reasons.clone(),
            pure_leaf: self.pure_leaf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_frame_dominates() {
        assert_eq!(join(EscapeTag::Local, EscapeTag::Local), EscapeTag::Local);
        assert_eq!(join(EscapeTag::Local, EscapeTag::Frame), EscapeTag::Frame);
        assert_eq!(join(EscapeTag::Frame, EscapeTag::Local), EscapeTag::Frame);
        assert_eq!(join(EscapeTag::Frame, EscapeTag::Frame), EscapeTag::Frame);
    }

    #[test]
    fn default_tag_is_local() {
        let s = ProcEscapeSummary::default();
        assert_eq!(s.tag("x"), EscapeTag::Local);
        assert!(!s.is_frame("anything"));
    }

    #[test]
    fn explicit_frame_tag_returns_frame() {
        let s = ProcEscapeSummary {
            tags: HashMap::from([("x".into(), EscapeTag::Frame)]),
            ..Default::default()
        };
        assert!(s.is_frame("x"));
        assert!(!s.is_frame("y"));
    }

    #[test]
    fn dynamic_barrier_forces_frame_for_all_names() {
        let s = ProcEscapeSummary {
            flags: EscapeFlags::DYNAMIC_BARRIER,
            tags: HashMap::from([("x".into(), EscapeTag::Local)]),
            ..Default::default()
        };
        // Even though ``x`` is recorded as Local, the barrier forces
        // every name to Frame.
        assert!(s.is_frame("x"));
        assert!(s.is_frame("not_in_tags"));
    }

    #[test]
    fn with_escapes_spills_named_vars() {
        let s = ProcEscapeSummary::default();
        let new = s.with_escapes(["a".into(), "b".into()], false);
        assert_eq!(new.tag("a"), EscapeTag::Frame);
        assert_eq!(new.tag("b"), EscapeTag::Frame);
        assert!(new.frame_needed);
    }

    #[test]
    fn with_escapes_pessimistic_sets_dynamic_barrier() {
        let s = ProcEscapeSummary::default();
        let new = s.with_escapes(std::iter::empty(), true);
        assert!(new.dynamic_barrier());
        assert!(new.frame_needed);
    }

    #[test]
    fn wants_frame_false_for_default_summary() {
        // Default summary: nothing escaped, no fallback — proc
        // can run with the frame elided.
        let s = ProcEscapeSummary::default();
        assert!(!s.wants_frame());
    }

    #[test]
    fn wants_frame_true_when_frame_needed() {
        // ``frame_needed`` alone is enough.
        let s = ProcEscapeSummary {
            frame_needed: true,
            ..Default::default()
        };
        assert!(s.wants_frame());
    }

    #[test]
    fn wants_frame_true_when_has_fallback() {
        // ``has_fallback`` alone is enough — eval-fallback resolves
        // names against the runtime frame.
        let s = ProcEscapeSummary {
            flags: EscapeFlags::HAS_FALLBACK,
            ..Default::default()
        };
        assert!(s.wants_frame());
    }

    #[test]
    fn reasons_for_local_var_returns_empty() {
        // ``Local`` vars never have a reason to surface — they
        // don't live in the frame.
        let s = ProcEscapeSummary::default();
        assert!(s.reasons_for("any_var").is_empty());
    }

    #[test]
    fn reasons_for_explicit_tag_reason_passes_through() {
        let mut tag_reasons = HashMap::new();
        tag_reasons.insert(
            "x".to_string(),
            vec![EscapeReason::with_detail(
                EscapeReasonKind::InfoExists,
                "info exists x",
            )],
        );
        let s = ProcEscapeSummary {
            tags: HashMap::from([("x".into(), EscapeTag::Frame)]),
            tag_reasons,
            ..Default::default()
        };
        let rs = s.reasons_for("x");
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].kind, EscapeReasonKind::InfoExists);
        assert_eq!(rs[0].detail, "info exists x");
    }

    #[test]
    fn reasons_for_synthesises_barrier_when_dynamic_barrier_set() {
        // No explicit tag_reason but proc is pessimistic — return
        // a synthesised ``Barrier`` reason.
        let s = ProcEscapeSummary {
            flags: EscapeFlags::DYNAMIC_BARRIER,
            barriers: vec![Barrier::with_detail(BarrierKind::Eval, "eval $body")],
            ..Default::default()
        };
        let rs = s.reasons_for("anything");
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].kind, EscapeReasonKind::Barrier);
        assert_eq!(rs[0].detail, "eval $body");
    }

    #[test]
    fn reasons_for_synthesises_placeholder_when_no_barriers_recorded() {
        // Pessimistic but no ``barriers`` populated — return a
        // single placeholder so consumers always get a structured
        // reason.
        let s = ProcEscapeSummary {
            flags: EscapeFlags::DYNAMIC_BARRIER,
            ..Default::default()
        };
        let rs = s.reasons_for("anything");
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].kind, EscapeReasonKind::Barrier);
        assert!(rs[0].detail.is_empty());
    }

    #[test]
    fn barrier_with_detail_constructor() {
        let b = Barrier::with_detail(BarrierKind::Upvar, "upvar $level");
        assert_eq!(b.kind, BarrierKind::Upvar);
        assert_eq!(b.detail, "upvar $level");
        assert!(b.range.is_none());
    }

    #[test]
    fn escape_reason_constructors() {
        let r = EscapeReason::new(EscapeReasonKind::CalleeUpvar);
        assert_eq!(r.kind, EscapeReasonKind::CalleeUpvar);
        assert!(r.detail.is_empty());

        let r2 = EscapeReason::with_detail(EscapeReasonKind::EvalReference, "eval body refs x");
        assert_eq!(r2.detail, "eval body refs x");
    }

    #[test]
    fn non_empty_barriers_imply_dynamic_barrier_even_without_flag() {
        // A future
        // population path that pushes to ``barriers`` without also
        // setting the ``DYNAMIC_BARRIER`` flag must still cause
        // ``tag()`` / ``is_frame()`` / ``reasons_for()`` to treat
        // the proc as pessimistic.  Today ``dynamic_barrier()``
        // checks both — so this test catches any regression that
        // re-narrows it to flag-only.
        let s = ProcEscapeSummary {
            barriers: vec![Barrier::with_detail(BarrierKind::Eval, "eval $body")],
            ..Default::default()
        };
        assert!(s.dynamic_barrier());
        assert!(s.is_frame("any_var"));
        let rs = s.reasons_for("any_var");
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].kind, EscapeReasonKind::Barrier);
        assert_eq!(rs[0].detail, "eval $body");
    }

    #[test]
    fn with_escapes_preserves_barriers_and_reasons() {
        let mut tag_reasons = HashMap::new();
        tag_reasons.insert(
            "x".into(),
            vec![EscapeReason::new(EscapeReasonKind::InfoExists)],
        );
        let s = ProcEscapeSummary {
            barriers: vec![Barrier::new(BarrierKind::Info)],
            tag_reasons,
            ..Default::default()
        };
        let new = s.with_escapes(std::iter::empty(), false);
        assert_eq!(new.barriers.len(), 1);
        assert_eq!(new.tag_reasons.len(), 1);
    }
}
