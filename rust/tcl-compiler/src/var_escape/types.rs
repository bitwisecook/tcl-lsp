//! Types for the var-escape analysis (C33a).
//!
//! Mirrors `core/compiler/var_escape/_types.py`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use bitflags::bitflags;
use tcl_lexer::Span;

/// Why the analysis fell back to the dynamic-barrier path.
///
/// Mirrors Python's `BarrierKind` enum added by upstream commit
/// ``015288cf`` (PR #356).  Each kind names a distinct *reason* so
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
/// Mirrors Python's `Barrier` dataclass added by ``015288cf``.
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
/// answer "why is `x` `Frame`?" precisely.  Mirrors Python's
/// `EscapeReasonKind` enum added by ``015288cf``.
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
///
/// Mirrors Python's `EscapeReason` dataclass added by ``015288cf``.
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
        /// A non-frameless `IRCall` with a statically resolvable
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
    ///
    /// Mirrors `local_slots` on Python's `ProcEscapeSummary`
    /// (added by upstream commit ``957dc1f6``).
    pub local_slots: BTreeMap<String, u32>,
    /// Recorded barrier triggers.  Each barrier carries its kind +
    /// detail + source range so callers (codegen, compiler
    /// explorer, LSP) can narrow / explain rather than treat every
    /// pessimistic-frame proc as opaque.  Always consistent with
    /// `dynamic_barrier`: the latter is `true` iff `barriers` is
    /// non-empty *or* the dynamic-barrier flag was set explicitly.
    /// Empty for procs the analysis proved frame-elidable.
    /// Mirrors Python's `ProcEscapeSummary.barriers` field added
    /// by upstream commit ``015288cf`` (PR #356).
    ///
    /// **Status:** the analysis pipeline (`propagation` /
    /// `cfg_propagation` / `interprocedural`) does not yet
    /// populate this field — that's tracked as a deferred
    /// follow-up.  Consumers see an empty `Vec` until the
    /// population work lands; `reasons_for` synthesises a single
    /// `Barrier` reason from `dynamic_barrier()` so LSP / explorer
    /// surfaces still get a structured fallback.
    pub barriers: Vec<Barrier>,
    /// Per-variable escape reasons.  `tag_reasons[name]` lists
    /// every trigger that contributed to `tags[name] == Frame`.
    /// An empty `Vec` (or missing key) means the analysis didn't
    /// record a specific reason — typically because the var
    /// stayed `Local` or because a barrier-induced escape is
    /// implied by `barriers` rather than explicit per-name
    /// (`Barrier` reasons are usually synthesised on demand by
    /// [`Self::reasons_for`]).  Mirrors Python's
    /// `ProcEscapeSummary.tag_reasons` field added by ``015288cf``.
    ///
    /// **Status:** like `barriers`, this is a deferred follow-up
    /// for the population pipeline; the type surface lands here
    /// so downstream consumers (LSP, compiler explorer) can adopt
    /// it incrementally.
    pub tag_reasons: HashMap<String, Vec<EscapeReason>>,
}

impl ProcEscapeSummary {
    /// True if the whole-proc dynamic-barrier marker is set.
    #[must_use]
    pub fn dynamic_barrier(&self) -> bool {
        self.flags.dynamic_barrier()
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
    /// `IRCall` with a statically resolvable command word. Whether
    /// that reaches the eval fallback depends on whether the
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
    /// catches every hazard.  Mirrors Python's
    /// `ProcEscapeSummary.wants_frame` property added by upstream
    /// commit ``015288cf`` (PR #356).
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
    /// frame; no reason to surface.  Mirrors Python's
    /// `ProcEscapeSummary.reasons_for` method added by ``015288cf``.
    ///
    /// **Status:** matches Python's behaviour for the synthesise-
    /// from-barriers fallback path; explicit per-name reasons are
    /// returned when the population follow-up populates
    /// `tag_reasons`.  Until then this method returns either the
    /// empty `Vec` (var is `Local`) or a single placeholder
    /// `Barrier` reason (var is `Frame` due to `dynamic_barrier`).
    #[must_use]
    pub fn reasons_for(&self, name: &str) -> Vec<EscapeReason> {
        if !self.is_frame(name) {
            return Vec::new();
        }
        if let Some(explicit) = self.tag_reasons.get(name) {
            if !explicit.is_empty() {
                return explicit.clone();
            }
        }
        if self.dynamic_barrier() {
            // Synthesise from proc-level barriers when present —
            // matches Python's fallback so the LSP / compiler
            // explorer has structured detail to surface even when
            // nothing specific was recorded for this var.
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
        for name in extra_escaped {
            new_tags.insert(name, EscapeTag::Frame);
        }
        let new_pessimistic = self.dynamic_barrier() || pessimistic;
        let new_frame_needed = new_pessimistic || new_tags.values().any(|t| *t == EscapeTag::Frame);
        let mut new_flags = self.flags;
        new_flags.set(EscapeFlags::DYNAMIC_BARRIER, new_pessimistic);
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
        }
    }

    /// Return a new summary with *slots* installed as the
    /// `local_slots` field.  Mirrors Python's
    /// `ProcEscapeSummary.with_local_slots` (added by upstream
    /// commit ``957dc1f6`` for the slot-resolution pass).
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
        // a synthesised ``Barrier`` reason mirroring Python's
        // fallback path.
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
