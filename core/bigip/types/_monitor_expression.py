"""Typed BIG-IP monitor-expression value.

The ``monitor`` property on pools, nodes, and GTM objects is more than
a single reference — it's a tiny expression grammar:

    monitor default
    monitor /Common/http
    monitor /Common/http and /Common/tcp
    monitor min 2 of { /Common/gateway_icmp /Common/http /Common/http2 }

This module models the parsed shape so consumers can ask structured
questions ("which monitors are required?", "how many of the listed
monitors must pass?", "is this a min-of expression?") without
re-implementing the grammar each time, and the rewrite layer can
update the monitor set safely (preserving the user's original
operator + ordering when no monitors changed).

The class lives in :mod:`core.bigip.types` so it's available to every
consumer (parser, projection, query, LSP) the same way
:class:`Destination` and :class:`Network` are.  The matching
:class:`core.bigip.registry.value_specs.MonitorExpressionSpec` wires
it into the registry.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

# Modes mirror the TMSH-documented monitor-expression forms.
MonitorMode = Literal["default", "none", "single", "all", "min-of"]


@dataclass(frozen=True, slots=True)
class MonitorExpression:
    """A parsed ``monitor`` expression.

    ``mode`` picks the grammatical shape:

    - ``"default"`` — ``monitor default`` keyword (use the parent
      object's default monitor selection).
    - ``"none"`` — ``monitor none`` (suppress monitoring entirely).
    - ``"single"`` — one bare monitor reference
      (``monitor /Common/http``).
    - ``"all"`` — ``monitor M1 and M2 [and M3 ...]`` (every listed
      monitor must report up).
    - ``"min-of"`` — ``monitor min N of { M1 M2 [M3 ...] }`` (at
      least ``minimum`` of the listed monitors must report up).

    ``monitors`` is the ordered list of monitor full-paths (empty for
    the ``default`` and ``none`` modes).  ``minimum`` is the threshold
    for the ``min-of`` mode and ``None`` for every other mode.  The
    original spelling is preserved on ``raw`` so a round trip through
    the rewrite layer doesn't change operator spacing or formatting
    when no monitor membership changed.
    """

    mode: MonitorMode = "default"
    monitors: tuple[str, ...] = ()
    minimum: int | None = None
    raw: str = ""

    @classmethod
    def try_parse(cls, text: str) -> "MonitorExpression | None":
        """Parse a TMSH monitor expression.  Returns ``None`` when the
        text doesn't match any of the four grammatical forms.

        The parser intentionally stays at the syntactic level — it
        doesn't validate that the referenced paths exist or that the
        ``min N of`` count is sensible relative to the list length;
        that's the job of the surrounding spec / diagnostic layer.
        """
        if text is None:
            return None
        stripped = text.strip()
        if not stripped:
            return None
        # ``monitor default`` / ``monitor none`` — the bare keywords.
        if stripped == "default":
            return cls(mode="default", raw=stripped)
        if stripped == "none":
            return cls(mode="none", raw=stripped)
        # ``min N of { M1 M2 ... }`` — the threshold form.
        if stripped.startswith("min "):
            return cls._parse_min_of(stripped)
        # Everything else is a chain of monitor references joined by
        # ``and`` (and possibly just one reference).  ``and`` tokens
        # are bareword separators in TMSH so a simple split is
        # sufficient.
        return cls._parse_and_chain(stripped)

    @classmethod
    def _parse_min_of(cls, text: str) -> "MonitorExpression | None":
        # Strip the leading ``min`` keyword and the trailing braced
        # list; extract N from between.
        rest = text[len("min ") :].lstrip()
        # ``rest`` looks like ``N of { M1 M2 ... }``.
        of_idx = rest.find(" of ")
        if of_idx < 0:
            return None
        try:
            minimum = int(rest[:of_idx].strip())
        except ValueError:
            return None
        body = rest[of_idx + len(" of ") :].lstrip()
        if not body.startswith("{") or not body.rstrip().endswith("}"):
            return None
        inner = body.strip()[1:-1].strip()
        monitors = tuple(tok for tok in inner.split() if tok)
        if not monitors:
            return None
        return cls(mode="min-of", monitors=monitors, minimum=minimum, raw=text)

    @classmethod
    def _parse_and_chain(cls, text: str) -> "MonitorExpression | None":
        # Split on whitespace-delimited ``and`` tokens.  TMSH never
        # quotes monitor paths so a token-level split is safe (paths
        # contain ``/``, ``_``, ``-`` but never spaces).
        parts = text.split()
        monitors: list[str] = []
        for i, token in enumerate(parts):
            if i % 2 == 1:
                # Expect ``and`` between every pair of monitor refs.
                if token != "and":
                    return None
                continue
            if not token:
                return None
            monitors.append(token)
        if not monitors:
            return None
        if len(monitors) == 1:
            return cls(mode="single", monitors=tuple(monitors), raw=text)
        return cls(mode="all", monitors=tuple(monitors), raw=text)

    @property
    def is_default(self) -> bool:
        return self.mode == "default"

    @property
    def is_none(self) -> bool:
        return self.mode == "none"

    @property
    def full_path(self) -> str:
        """Canonical scalar spelling of the expression.

        For ``mode="single"`` this is the bare monitor path (so
        ``.monitor.full-path`` queries against a single-monitor
        expression keep returning the path, the historical PathRef
        contract).  For multi-monitor expressions it returns the
        full canonical render (``"/Common/http and /Common/tcp"``,
        ``"min 2 of { ... }"``) — the same string a user sees in
        the TMSH config.
        """
        if self.mode == "single" and self.monitors:
            return self.monitors[0]
        if self.mode == "default":
            return "default"
        if self.mode == "none":
            return "none"
        return str(self)

    @property
    def name(self) -> str:
        """Leaf name of the (first) referenced monitor — convenience
        for ``.monitor.name`` queries that want the short form."""
        if not self.monitors:
            return ""
        return self.monitors[0].rsplit("/", 1)[-1]

    def __iter__(self):
        """Iterating the expression yields each referenced monitor
        path — so ``.monitor[]`` in the query DSL produces one item
        per monitor in the expression (empty for ``default`` /
        ``none`` modes)."""
        return iter(self.monitors)

    def __len__(self) -> int:
        return len(self.monitors)

    def __bool__(self) -> bool:
        # Empty / default / none modes still behave as truthy values
        # when the user explicitly compares against them; falsey-ness
        # is reserved for the empty-string case the projection emits
        # when ``monitor`` was unset on the model.
        return self.mode != "default" or bool(self.raw)

    def references(self) -> tuple[str, ...]:
        """Return the full-paths this expression references.

        Provides the integration point for the registry's reference
        layer: the spec's ``references()`` method enumerates these as
        :class:`core.bigip.registry.value_specs.Reference` records so
        the graph and LSP layers see every monitor reference without
        re-parsing the expression.
        """
        return self.monitors

    def __str__(self) -> str:
        # Prefer the original spelling when the value parsed from
        # text and the structured fields still match — that preserves
        # spacing / comments on no-op round trips.  A
        # ``dataclasses.replace`` that mutates ``monitors`` /
        # ``mode`` / ``minimum`` would leave raw stale, so we
        # re-parse raw and compare to the current structured shape;
        # any mismatch falls through to the canonical render so the
        # change is reflected in the output.
        if self.raw:
            reparsed = MonitorExpression.try_parse(self.raw)
            if (
                reparsed is not None
                and reparsed.mode == self.mode
                and reparsed.monitors == self.monitors
                and reparsed.minimum == self.minimum
            ):
                return self.raw
        if self.mode == "default":
            return "default"
        if self.mode == "none":
            return "none"
        if self.mode == "single":
            return self.monitors[0] if self.monitors else ""
        if self.mode == "all":
            return " and ".join(self.monitors)
        if self.mode == "min-of":
            inner = " ".join(self.monitors)
            return f"min {self.minimum} of {{ {inner} }}"
        return ""


__all__ = ["MonitorExpression", "MonitorMode"]
