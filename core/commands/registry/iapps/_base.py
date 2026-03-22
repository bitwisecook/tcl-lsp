"""Per-package registry for iApps and tmsh command definitions."""

from __future__ import annotations

from .._base import CommandDef, make_registry  # noqa: F401
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity

_IAPPS_ONLY = frozenset({"f5-iapps"})
# tmsh commands are available in both iApps and standalone tmsh scripts.
_TMSH_DIALECTS = frozenset({"f5-iapps", "f5-tmsh"})

_SOURCE_TMSH = "F5 TMSH Scripting Reference"

_REGISTRY, register = make_registry()


def _tmsh_spec(
    name: str,
    summary: str,
    synopsis: str = "",
    *,
    min_args: int = 0,
    max_args: int | None = None,
) -> CommandSpec:
    """Build a CommandSpec for a tmsh:: command."""
    if not synopsis:
        synopsis = f"{name} ?arg ...?"
    arity = Arity(min=min_args) if max_args is None else Arity(min=min_args, max=max_args)
    return CommandSpec(
        name=name,
        dialects=_TMSH_DIALECTS,
        hover=HoverSnippet(
            summary=summary,
            synopsis=(synopsis,),
            source=_SOURCE_TMSH,
        ),
        forms=(FormSpec(kind=FormKind.DEFAULT, synopsis=synopsis),),
        validation=ValidationSpec(arity=arity),
    )
