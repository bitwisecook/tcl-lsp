"""Taint lattice types, colour constants, and source/sanitiser helpers."""

from __future__ import annotations

from dataclasses import dataclass

from ...commands.registry.runtime import TAINT_HINTS, TYPE_HINTS
from ...commands.registry.signatures import Arity
from ...commands.registry.taint_hints import TaintColour
from ...commands.registry.type_hints import CommandTypeHint, SubcommandTypeHint
from ..types import TclType

_ALL_COLOURS = (
    TaintColour.TAINTED
    | TaintColour.PATH_PREFIXED
    | TaintColour.NON_DASH_PREFIXED
    | TaintColour.CRLF_FREE
    | TaintColour.SHELL_ATOM
    | TaintColour.LIST_CANONICAL
    | TaintColour.REGEX_LITERAL
    | TaintColour.PATH_NORMALISED
    | TaintColour.HEADER_TOKEN_SAFE
    | TaintColour.HTML_ESCAPED
    | TaintColour.URL_ENCODED
    | TaintColour.IP_ADDRESS
    | TaintColour.PORT
    | TaintColour.FQDN
)

# Colours that prove a value cannot start with '-' (option injection safe).
_T102_SAFE = (
    TaintColour.PATH_PREFIXED
    | TaintColour.NON_DASH_PREFIXED
    | TaintColour.IP_ADDRESS
    | TaintColour.PORT
    | TaintColour.FQDN
)

# Colours that mitigate CRLF/header/log injection.
_CRLF_SAFE = TaintColour.CRLF_FREE | TaintColour.IP_ADDRESS | TaintColour.PORT | TaintColour.FQDN


@dataclass(frozen=True, slots=True)
class TaintLattice:
    """Colour-aware taint lattice.

    * ``tainted=False`` → untainted (bottom).
    * ``tainted=True, colour=…`` → tainted with the given colour set.

    Colours are properties of the tainted value.  At join points,
    colours *intersect*: only properties shared by all incoming
    operands survive.
    """

    tainted: bool
    colour: TaintColour = TaintColour(0)

    @staticmethod
    def untainted() -> TaintLattice:
        return _UNTAINTED

    @staticmethod
    def of(colour: TaintColour) -> TaintLattice:
        return TaintLattice(tainted=True, colour=colour)


_UNTAINTED = TaintLattice(tainted=False)
_TAINTED = TaintLattice(tainted=True, colour=TaintColour.TAINTED)


def taint_join(a: TaintLattice, b: TaintLattice) -> TaintLattice:
    """Join two taint lattice values.

    * Taint dominates: if either is tainted, the result is tainted.
    * Colours intersect: only properties common to both survive.
    """
    if not a.tainted and not b.tainted:
        return _UNTAINTED
    if not a.tainted:
        return b
    if not b.tainted:
        return a
    # Both tainted: intersect colours.
    merged_colour = a.colour & b.colour
    if merged_colour == TaintColour.TAINTED:
        return _TAINTED
    return TaintLattice(tainted=True, colour=merged_colour)


# Interprocedural taint summaries

_BASIS_ORDER = (
    "generic",
    "path",
    "non_dash",
    "crlf_free",
    "shell_atom",
    "list_canonical",
    "regex_literal",
    "path_normalised",
    "header_token_safe",
    "html_escaped",
    "url_encoded",
    "ip",
    "port",
    "fqdn",
)
_BASIS_LATTICES: dict[str, TaintLattice] = {
    "generic": _TAINTED,
    "path": TaintLattice.of(TaintColour.TAINTED | TaintColour.PATH_PREFIXED),
    "non_dash": TaintLattice.of(TaintColour.TAINTED | TaintColour.NON_DASH_PREFIXED),
    "crlf_free": TaintLattice.of(TaintColour.TAINTED | TaintColour.CRLF_FREE),
    "shell_atom": TaintLattice.of(TaintColour.TAINTED | TaintColour.SHELL_ATOM),
    "list_canonical": TaintLattice.of(TaintColour.TAINTED | TaintColour.LIST_CANONICAL),
    "regex_literal": TaintLattice.of(TaintColour.TAINTED | TaintColour.REGEX_LITERAL),
    "path_normalised": TaintLattice.of(TaintColour.TAINTED | TaintColour.PATH_NORMALISED),
    "header_token_safe": TaintLattice.of(TaintColour.TAINTED | TaintColour.HEADER_TOKEN_SAFE),
    "html_escaped": TaintLattice.of(TaintColour.TAINTED | TaintColour.HTML_ESCAPED),
    "url_encoded": TaintLattice.of(TaintColour.TAINTED | TaintColour.URL_ENCODED),
    "ip": TaintLattice.of(TaintColour.TAINTED | TaintColour.IP_ADDRESS),
    "port": TaintLattice.of(TaintColour.TAINTED | TaintColour.PORT),
    "fqdn": TaintLattice.of(TaintColour.TAINTED | TaintColour.FQDN),
}


@dataclass(frozen=True, slots=True)
class ProcTaintSummary:
    """Context-insensitive return-taint transfer summary for one procedure."""

    qualified_name: str
    params: tuple[str, ...]
    arity: Arity
    return_base: TaintLattice
    return_by_param_basis: tuple[tuple[str, tuple[TaintLattice, ...]], ...]

    def scenario(self, param: str, basis: str) -> TaintLattice:
        idx = _BASIS_ORDER.index(basis)
        for p_name, values in self.return_by_param_basis:
            if p_name == param and idx < len(values):
                return values[idx]
        return _UNTAINTED


# Sanitisers — commands that produce a safe fixed-type result

_SANITISER_RETURN_TYPES = frozenset({TclType.INT, TclType.BOOLEAN})


def _is_sanitiser(command: str, args: tuple[str, ...]) -> bool:
    """Return True if *command* produces a safe fixed-type result."""
    hint = TYPE_HINTS.get(command)
    if hint is None:
        return False
    if isinstance(hint, SubcommandTypeHint):
        if not args:
            return False
        sub_hint = hint.subcommands.get(args[0])
        if sub_hint is None or sub_hint.return_type is None:
            return False
        return sub_hint.return_type in _SANITISER_RETURN_TYPES
    if isinstance(hint, CommandTypeHint):
        if hint.return_type is None:
            return False
        return hint.return_type in _SANITISER_RETURN_TYPES
    return False


def _taint_source_colour(command: str, args: tuple[str, ...]) -> TaintLattice | None:
    """Return the taint lattice for a source command, or None if not a source.

    All taint-source metadata is defined via ``taint_hints()`` on the
    corresponding :class:`CommandDef` and collected into ``TAINT_HINTS``
    at import time.
    """

    def _augment_source_colours(
        cmd: str,
        cmd_args: tuple[str, ...],
        colour: TaintColour,
    ) -> TaintColour:
        """Add conservative derived properties for known source shapes."""
        out = colour
        if bool(out & TaintColour.PATH_PREFIXED):
            out |= TaintColour.NON_DASH_PREFIXED
        if bool(out & (TaintColour.IP_ADDRESS | TaintColour.PORT | TaintColour.FQDN)):
            out |= TaintColour.NON_DASH_PREFIXED | TaintColour.CRLF_FREE | TaintColour.SHELL_ATOM

        return out

    hint = TAINT_HINTS.get(command)
    if hint is None or hint.source is None:
        return None

    # Subcommand gating: if source_subcommands is set, only taint when
    # the first argument (subcommand) is in the allowed set.
    if hint.source_subcommands is not None:
        if not args or args[0] not in hint.source_subcommands:
            return None

    n_args = len(args)
    # Try exact arity match first.
    for key, colour in hint.source.items():
        if key is None:
            continue  # skip catch-all for now
        if key.accepts(n_args):
            return TaintLattice.of(_augment_source_colours(command, args, colour))
    # Try catch-all.
    catch_all = hint.source.get(None)
    if catch_all is not None:
        return TaintLattice.of(_augment_source_colours(command, args, catch_all))
    # Has hint.source but no arity match — not a source for this arity.
    return None
