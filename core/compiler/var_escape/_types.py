"""Types for the var-escape analysis."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Iterable


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
    # ``core.compiler.ssa.SSAValueKey``.  The per-name ``tags``
    # field is the join over this dict and is what codegen consumes.
    ssa_tags: dict[tuple[str, int], EscapeTag] = field(default_factory=dict)

    def tag(self, name: str) -> EscapeTag:
        """Return the tag for ``name`` (defaults to ``LOCAL``)."""
        if self.dynamic_barrier:
            return EscapeTag.FRAME
        return self.tags.get(name, EscapeTag.LOCAL)

    def is_frame(self, name: str) -> bool:
        """Shorthand: does ``name`` need to live in the runtime frame?"""
        return self.tag(name) is EscapeTag.FRAME

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
        structure.
        """
        new_tags = dict(self.tags)
        for name in extra_escaped:
            new_tags[name] = EscapeTag.FRAME
        new_pessimistic = self.dynamic_barrier or pessimistic
        new_frame_needed = new_pessimistic or any(
            tag is EscapeTag.FRAME for tag in new_tags.values()
        )
        # pure_leaf is invalidated by any added escape — once a var
        # spills to FRAME the proc is no longer purely-local.
        new_pure_leaf = self.pure_leaf and not extra_escaped and not new_pessimistic
        return ProcEscapeSummary(
            tags=new_tags,
            dynamic_barrier=new_pessimistic,
            frame_needed=new_frame_needed,
            upvar_source_names=self.upvar_source_names,
            unbounded_upvar_source=self.unbounded_upvar_source,
            direct_callees=self.direct_callees,
            has_fallback=self.has_fallback,
            has_call_fallback=self.has_call_fallback,
            ssa_tags=dict(self.ssa_tags),
            pure_leaf=new_pure_leaf,
        )
