"""Types for the var-escape analysis."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import TYPE_CHECKING, Iterable

if TYPE_CHECKING:
    from shared.diagnostic import Range


class EscapeTag(Enum):
    """Where a Tcl variable must live at runtime.

    ``LOCAL`` — only accessed through statically resolved positions; the
    WASM local slot is the single source of truth.

    ``FRAME`` — must live in the runtime frame so the interpreter (or an
    ``upvar`` alias) can read and write it by name.
    """

    LOCAL = "local"
    FRAME = "frame"


def join(a: EscapeTag, b: EscapeTag) -> EscapeTag:
    """Join operator on the lattice: FRAME dominates."""
    if a is EscapeTag.FRAME or b is EscapeTag.FRAME:
        return EscapeTag.FRAME
    return EscapeTag.LOCAL


class BarrierKind(Enum):
    """Why the analysis fell back to the dynamic-barrier path.

    Each kind names a distinct *reason* so codegen, the LSP, and the
    compiler explorer can narrow / explain rather than treat every
    pessimistic-frame proc as opaque.

    ``INFO`` — frame-inspecting ``info`` subcommand (``info level``,
    ``info frame``, ``info vars``, ``info locals``, ``info coroutine``,
    ``info errorstack``), or an unknown / dynamic ``info`` subcommand.

    ``EVAL`` — ``eval``/``source``/``subst`` with a body the analysis
    can't statically scan (dynamic body, parse failure, lowering
    failure).

    ``UPVAR`` — ``upvar`` with a dynamic level or unbounded source,
    or a non-#0 ``uplevel`` whose body could touch caller-frame names.

    ``DYN_NAME`` — assignment / increment / unset whose first arg is
    a dynamic name the analysis can't resolve to a literal target
    (``set $varname value`` etc.) — the fallback escapes every known
    proc-local.

    ``EXPAND`` — ``{*}``-word expansion in an unknown command call,
    where argument-index analysis can't tell where the name landed.

    ``UNKNOWN`` — any other barrier shape we don't yet classify
    (catch-all so the field stays exhaustive).
    """

    INFO = "info"
    EVAL = "eval"
    UPVAR = "upvar"
    DYN_NAME = "dyn_name"
    EXPAND = "expand"
    UNKNOWN = "unknown"


@dataclass(frozen=True, slots=True)
class Barrier:
    """A single barrier trigger recorded by the analysis.

    ``kind`` identifies the category (see :class:`BarrierKind`).
    ``detail`` is a short human-readable explanation, intended to be
    surfaced verbatim in compiler-explorer hovers and LSP hints
    (e.g. ``"info level"``, ``"eval $body (dynamic)"``,
    ``"set $varname value"``).  ``range`` is the source span of the
    triggering construct when the analysis pass had it available — None
    when the barrier was synthesised from an inferred / nested IR shape.
    """

    kind: BarrierKind
    detail: str = ""
    range: "Range | None" = None


class EscapeReasonKind(Enum):
    """Why a specific variable was tagged ``FRAME``.

    Per-name companion to :class:`BarrierKind`.  A barrier is a
    proc-level fact ("this proc went pessimistic"); an escape reason
    is a per-variable fact ("this var is FRAME because …") — they
    overlap (a barrier *implies* every-known-var is FRAME) but the
    finer split lets the LSP / compiler explorer answer "why is
    ``x`` FRAME?" precisely.

    ``BARRIER`` — proc went pessimistic; the var is FRAME by virtue of
    being in the proc.  Pair with the proc's :class:`Barrier` for
    detail.

    ``INFO_EXISTS`` — ``[info exists <name>]`` references this var by
    literal name.  The interpreter resolves the lookup against the
    runtime frame, so the var must live there.

    ``UPVAR_SOURCE`` — this name is the *source* of an ``upvar``
    declaration in this proc.  Caller-side aliasing requires the
    name to be in the runtime frame.

    ``CALLEE_UPVAR`` — interprocedural: a callee upvars this name
    from our frame, so we have to spill it.

    ``EVAL_REFERENCE`` — an ``eval``/``source``/``subst`` body
    references this name (``$<name>`` or ``set <name> ...``).

    ``DYN_NAME_RESOLVED`` — a dynamic-name command (``set $v ...``)
    resolved to this literal target via in-body literal analysis.

    ``ESCAPE_ALL_KNOWN`` — fallback path spilled every known
    proc-local because no narrower escape set could be proved.
    """

    BARRIER = "barrier"
    INFO_EXISTS = "info_exists"
    UPVAR_SOURCE = "upvar_source"
    CALLEE_UPVAR = "callee_upvar"
    EVAL_REFERENCE = "eval_reference"
    DYN_NAME_RESOLVED = "dyn_name_resolved"
    ESCAPE_ALL_KNOWN = "escape_all_known"


@dataclass(frozen=True, slots=True)
class EscapeReason:
    """A single per-variable escape reason recorded by the analysis."""

    kind: EscapeReasonKind
    detail: str = ""
    range: "Range | None" = None


@dataclass(frozen=True, slots=True)
class ProcEscapeSummary:
    """Per-procedure escape classification.

    ``tags`` maps variable name to its escape tag. Names not present
    default to ``EscapeTag.LOCAL`` — the caller may treat the dict as
    "what needs spilling".

    ``dynamic_barrier`` is set when the analysis encountered a
    construct whose name-reference set cannot be bounded (``eval
    $body``, ``uplevel 1``, ``info level``, ``{*}$dynamic`` in an
    unknown call, etc.). In that case every variable in the proc is
    effectively ``FRAME`` regardless of what ``tags`` contains, and
    the codegen must fall back to the sync-everything path.

    ``frame_needed`` is a convenience flag for codegen: True if the
    proc needs a runtime frame at all. Equivalent to
    ``dynamic_barrier or any(tag is FRAME for tag in tags.values())``.

    ``upvar_source_names`` is the set of literal variable names this
    proc (or any of its transitive callees once the interprocedural
    pass has run) names as the *source* of a caller-frame ``upvar``.
    A caller must treat any of its local vars whose names appear in a
    callee's ``upvar_source_names`` as ``FRAME`` — the callee aliases
    them by name from the frame. ``unbounded_upvar_source`` is True
    when the source set can't be enumerated (dynamic source name,
    pessimistic callee, …) — callers must spill every local.

    ``direct_callees`` is the set of qualified proc names this proc
    calls with statically known arguments, used by the interprocedural
    pass to drive the fixpoint.
    """

    tags: dict[str, EscapeTag] = field(default_factory=dict)
    dynamic_barrier: bool = False
    frame_needed: bool = False
    upvar_source_names: frozenset[str] = frozenset()
    unbounded_upvar_source: bool = False
    direct_callees: frozenset[str] = frozenset()
    has_fallback: bool = False
    # Each barrier carries its kind + detail + source range so callers
    # (codegen, compiler explorer, LSP) can narrow / explain rather
    # than treat every pessimistic-frame proc as opaque.  Always
    # consistent with ``dynamic_barrier``: the latter is True iff
    # ``barriers`` is non-empty.  Empty for procs the analysis proved
    # frame-elidable.
    barriers: tuple[Barrier, ...] = ()
    # Per-variable escape reasons.  ``tag_reasons[name]`` lists every
    # trigger that contributed to ``tags[name] == FRAME``.  Empty
    # tuple (or missing key) means the analysis didn't record a
    # specific reason — typically because the var stayed LOCAL or
    # because a barrier-induced escape is implied by ``barriers``
    # rather than explicit per-name (``BARRIER`` reasons are usually
    # synthesised on demand by ``reasons_for(name)``).
    tag_reasons: dict[str, tuple[EscapeReason, ...]] = field(default_factory=dict)
    # S3.4: True when the proc is "pure leaf" — no dynamic_barrier,
    # no frame_needed, no has_fallback, no global mutation, no
    # upvar/uplevel/info, and every direct callee is itself
    # pure_leaf.  S4 (inlining) reads this flag as the safety
    # predicate for IR-level inlining: a pure_leaf proc can be
    # spliced into the caller's IRBlock without changing
    # observable behaviour.  Default False — any proc that
    # touches a runtime side-effect or that hasn't been classified
    # by the analysis stays opaque.
    pure_leaf: bool = False
    # True if the intraprocedural pass saw a non-frameless ``IRCall``
    # with a statically resolvable command word.  Whether that reaches
    # the eval fallback depends on whether the callee is a compiled
    # proc — only the interprocedural pass can tell.  Codegen does
    # NOT read this field directly; it reads ``has_fallback`` after
    # the interprocedural downgrade has run.
    has_call_fallback: bool = False
    # Per-SSA-version escape tags, populated by the flow-sensitive
    # CFG+SSA propagation.  Empty when the analysis was driven from
    # an IR-only source (no CompilationUnit).  ``ssa_tags`` is keyed
    # by ``(var_name, ssa_version)`` — see
    # ``compiler.ssa.SSAValueKey``.  The per-name ``tags``
    # field is the join over this dict and is what codegen consumes.
    ssa_tags: dict[tuple[str, int], EscapeTag] = field(default_factory=dict)
    # Phase 7: compile-time slot indices for proc-locals that pass
    # the slot-resolution eligibility check.  Populated by the
    # ``assign_local_slots`` pass after escape analysis runs;
    # consumed by the WASM codegen's variable read/write paths
    # (``_emit_var_write_obj_impl`` / ``_emit_var_read_obj_lenient``)
    # to emit ``frame_local_set_at(idx)`` / ``frame_local_at(idx)``
    # instead of the name-keyed ``tcl_local_set`` / ``tcl_local_get``.
    # Empty for procs that aren't eligible (dynamic_barrier, upvar /
    # global / variable / info exists / trace add variable on a
    # local, dynamic name access, nested eval / uplevel / apply).
    local_slot_indices: dict[str, int] = field(default_factory=dict)

    def tag(self, name: str) -> EscapeTag:
        """Return the tag for ``name`` (defaults to ``LOCAL``)."""
        if self.dynamic_barrier:
            return EscapeTag.FRAME
        return self.tags.get(name, EscapeTag.LOCAL)

    def is_frame(self, name: str) -> bool:
        """Shorthand: does ``name`` need to live in the runtime frame?"""
        return self.tag(name) is EscapeTag.FRAME

    def reasons_for(self, name: str) -> tuple[EscapeReason, ...]:
        """Return the per-variable escape reasons for ``name``.

        When the proc went pessimistic (``dynamic_barrier=True``) and
        no specific per-name reason was recorded, synthesise one
        ``BARRIER`` reason per proc-level :class:`Barrier` so callers
        can answer "why is ``x`` FRAME?" with structured detail
        instead of "because everything is."

        When the var is ``LOCAL`` returns an empty tuple — the var
        is not in the frame; no reason to surface.
        """
        if not self.is_frame(name):
            return ()
        explicit = self.tag_reasons.get(name, ())
        if explicit:
            return explicit
        if self.dynamic_barrier:
            # Synthesise from proc-level barriers so the LSP / explorer
            # has structured detail to surface even when nothing
            # specific was recorded for this var.
            return tuple(
                EscapeReason(kind=EscapeReasonKind.BARRIER, detail=b.detail, range=b.range)
                for b in self.barriers
            ) or (EscapeReason(kind=EscapeReasonKind.BARRIER),)
        return ()

    @property
    def wants_frame(self) -> bool:
        """Single source of truth for "this proc needs a runtime frame".

        Combines every analysis-known reason a proc body can't run with
        the frame elided:

        * ``frame_needed`` — at least one var is FRAME (escapes to a
          callee's ``upvar`` source, or the dynamic barrier flagged
          everything FRAME).
        * ``has_fallback`` — a statically resolvable command falls
          through to the eval fallback, which resolves names against
          the runtime frame.

        Codegen should consult this property instead of recomposing
        the same condition at each call site.  The property does NOT
        include codegen-time scans (e.g. ``info level`` references
        the analysis missed) — those are layered on top of this gate
        by the emitter's top-level ``wants_frame`` resolver until the
        analysis itself catches every hazard.
        """
        return self.frame_needed or self.has_fallback

    # -- PR #237 review: split predicates for separate proofs ---------
    #
    # ``pure_leaf`` is the union of every safety constraint we've
    # accumulated, so it's a safe-but-conservative gate for any
    # downstream pass.  The reviewer's point: each pass actually
    # needs a different proof.  These derived properties let
    # callers ask the precise question rather than over-rely on
    # the union flag.  The current implementations are conservative
    # equivalents — i.e. ``safe_to_inline`` is exactly ``pure_leaf``,
    # ``safe_to_dce`` and ``safe_for_frame_elision`` are
    # relaxations that rely on a smaller subset of the same fields.
    # Future tightening can refine each independently without
    # racing the others.

    @property
    def safe_to_inline(self) -> bool:
        """Can the proc body be physically relocated into the caller's
        IR without changing observable behaviour?

        Required (today, conservatively pulled from ``pure_leaf``):

        * No body-level ``upvar`` / ``uplevel`` / ``info level`` /
          ``info frame`` — those would observe the wrong frame
          after splice.
        * No ``eval`` / dynamic dispatch fallback that depends on
          the proc being registered as a runtime command.
        * Every direct callee is itself inline-safe (transitively).

        Implementation: identical to :attr:`pure_leaf`.  Future
        relaxation can split out the "callees are inline-safe"
        clause from the body-frame-observation clause if a
        downstream consumer needs a less restrictive predicate.
        """

        return self.pure_leaf

    @property
    def safe_to_dce(self) -> bool:
        """Is dead-store elimination on this proc's locals sound?

        Required: the proc's locals must be inaccessible to anything
        outside the body — no caller can read them through ``upvar``,
        no eval-fallback can introspect them through ``info``
        commands or arbitrary script (``[eval {set b}]`` reads our
        ``b`` slot through Tcl's frame resolution), no callee can
        reach into the frame.

        **PR #237 review — real relaxation.**  Previously this
        returned ``pure_leaf`` verbatim, which over-constrained DCE
        with the inlining-specific "every direct callee is itself
        pure_leaf" clause from the interprocedural fixpoint.  DCE
        doesn't care about *what* the callees do — only whether
        any observer (eval-fallback, upvar source, callee reaching
        through the frame) can read our locals.  The relaxation
        drops the IPA "callees pure_leaf" requirement but keeps
        every gate that protects locals from external observation:

        * ``not frame_needed`` — no var is FRAME-tagged (caller
          can't ``upvar`` into our frame, no callee triggered the
          dynamic_barrier downgrade).
        * ``not has_fallback`` — no static dispatch falls through
          to runtime eval, which would read locals by name.
        * ``not has_call_fallback`` — no compiled-proc dispatch
          could escape into a runtime path that introspects.
        * ``not unbounded_upvar_source`` — this proc doesn't
          itself ``upvar`` from a dynamically-named caller (which
          would alias one of the caller's slots to our writes).
        * ``not upvar_source_names`` — no ``upvar`` source set
          escapes our locals to a caller.
        """

        return (
            not self.frame_needed
            and not self.has_fallback
            and not self.has_call_fallback
            and not self.upvar_source_names
            and not self.unbounded_upvar_source
        )

    @property
    def safe_for_frame_elision(self) -> bool:
        """Can codegen omit the per-call ``tcl_frame_push`` /
        ``tcl_frame_pop`` for this proc?

        Required: the proc's frame is never observed externally
        (no upvar source from another proc, no caller's
        ``info level`` reading our depth, no eval-fallback that
        would need a real frame to resolve unqualified names).

        Implementation today: ``not frame_needed``.  This is a
        relaxation of ``pure_leaf`` — frame elision doesn't care
        whether the body itself does eval / has callees; only
        whether OUR frame matters externally.  ``frame_needed``
        already encodes exactly that question (it's set whenever
        any var or the dynamic barrier escapes).
        """

        return not self.frame_needed

    def with_escapes(
        self,
        extra_escaped: Iterable[str],
        *,
        pessimistic: bool = False,
    ) -> "ProcEscapeSummary":
        """Return a new summary with ``extra_escaped`` spilled to FRAME.

        Used by the interprocedural pass to fold callee-induced
        escapes (names a callee uses as ``upvar`` sources) into a
        caller's summary without mutating the originally computed
        structure.  Each added name is annotated with a
        ``CALLEE_UPVAR`` reason so downstream consumers can surface
        "this var is FRAME because a callee upvars it" rather than
        an unattributed FRAME tag.
        """
        new_tags = dict(self.tags)
        new_reasons = {k: v for k, v in self.tag_reasons.items()}
        callee_reason = EscapeReason(kind=EscapeReasonKind.CALLEE_UPVAR)
        for name in extra_escaped:
            new_tags[name] = EscapeTag.FRAME
            existing = new_reasons.get(name, ())
            if not any(r.kind is EscapeReasonKind.CALLEE_UPVAR for r in existing):
                new_reasons[name] = existing + (callee_reason,)
        new_pessimistic = self.dynamic_barrier or pessimistic
        new_frame_needed = new_pessimistic or any(
            tag is EscapeTag.FRAME for tag in new_tags.values()
        )
        new_barriers = self.barriers
        if pessimistic and not self.dynamic_barrier:
            new_barriers = self.barriers + (
                Barrier(kind=BarrierKind.UNKNOWN, detail="interprocedural"),
            )
        # pure_leaf is invalidated by any added escape — once a var
        # spills to FRAME the proc is no longer purely-local.
        new_pure_leaf = self.pure_leaf and not extra_escaped and not new_pessimistic
        return ProcEscapeSummary(
            tags=new_tags,
            dynamic_barrier=new_pessimistic,
            frame_needed=new_frame_needed,
            barriers=new_barriers,
            tag_reasons=new_reasons,
            upvar_source_names=self.upvar_source_names,
            unbounded_upvar_source=self.unbounded_upvar_source,
            direct_callees=self.direct_callees,
            has_fallback=self.has_fallback,
            has_call_fallback=self.has_call_fallback,
            ssa_tags=dict(self.ssa_tags),
            pure_leaf=new_pure_leaf,
            local_slot_indices=dict(self.local_slot_indices),
        )

    def with_local_slots(self, slot_indices: dict[str, int]) -> "ProcEscapeSummary":
        """Return a new summary carrying *slot_indices* as the
        compile-time slot mapping (Phase 7).  Frozen dataclass — must
        rebuild the structure rather than assign in place.
        """
        return ProcEscapeSummary(
            tags=dict(self.tags),
            dynamic_barrier=self.dynamic_barrier,
            frame_needed=self.frame_needed,
            upvar_source_names=self.upvar_source_names,
            unbounded_upvar_source=self.unbounded_upvar_source,
            direct_callees=self.direct_callees,
            has_fallback=self.has_fallback,
            has_call_fallback=self.has_call_fallback,
            barriers=self.barriers,
            tag_reasons={k: v for k, v in self.tag_reasons.items()},
            ssa_tags=dict(self.ssa_tags),
            pure_leaf=self.pure_leaf,
            local_slot_indices=dict(slot_indices),
        )
